//! The TLS 1.3 client handshake (RFC 8446 §4).
//!
//! The flow, once, so the code below reads as a sequence rather than a maze:
//!
//! ```text
//! -> ClientHello          (key share, offered suites, SNI)
//! <- ServerHello          (chosen suite, server key share)
//!    ... handshake keys exist from here on, everything below is encrypted
//! <- EncryptedExtensions
//! <- Certificate          (the chain)
//! <- CertificateVerify    (proves possession of the leaf's private key)
//! <- Finished             (proves the server saw the same transcript)
//! -> Finished
//!    ... application keys
//! ```
//!
//! The transcript hash running through all of it is what makes the handshake
//! tamper-evident: every secret is derived over it, so any modified message
//! yields different keys and the Finished check fails.

use std::io::{Read, Write};

use ring::agreement::{self, EphemeralPrivateKey, UnparsedPublicKey, X25519};
use ring::digest::{Context, SHA256};
use ring::rand::{SecureRandom, SystemRandom};

use super::cert::Certificate;
use super::codec::{Reader, Writer};
use super::key_schedule::{finished_mac, KeySchedule};
use super::msgs::{
    alert, ext, sig, CipherSuite, ContentType, HandshakeType, GROUP_X25519,
    SERVER_CERT_VERIFY_CONTEXT, VERSION_TLS12, VERSION_TLS13,
};
use super::record::{RecordCrypto, RecordLayer};
use super::verify::now_unix;
use super::{TlsConfig, TlsError};

/// The running hash of every handshake message, in order.
struct Transcript {
    ctx: Context,
}

impl Transcript {
    fn new() -> Self {
        Self {
            ctx: Context::new(&SHA256),
        }
    }

    fn add(&mut self, msg: &[u8]) {
        self.ctx.update(msg);
    }

    /// The hash so far. Cloning is required because a digest context is
    /// consumed on finish, and the transcript keeps growing after each use.
    fn hash(&self) -> Vec<u8> {
        self.ctx.clone().finish().as_ref().to_vec()
    }
}

/// Reassembles handshake messages, which may be split across records or
/// packed several to a record.
struct HandshakeReader {
    buf: Vec<u8>,
}

impl HandshakeReader {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    fn feed(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    /// Pops one complete message, returning its type and full encoding
    /// (header included, since the transcript covers that too).
    fn next(&mut self) -> Result<Option<(HandshakeType, Vec<u8>)>, TlsError> {
        if self.buf.len() < 4 {
            return Ok(None);
        }

        let len = u32::from_be_bytes([0, self.buf[1], self.buf[2], self.buf[3]]) as usize;
        if self.buf.len() < 4 + len {
            return Ok(None);
        }

        let raw: Vec<u8> = self.buf.drain(..4 + len).collect();
        let typ = HandshakeType::from_u8(raw[0])
            .ok_or(TlsError::Decode("unknown handshake message type"))?;

        Ok(Some((typ, raw)))
    }
}

/// Drives the handshake and returns a record layer with application keys
/// installed.
pub fn connect<S: Read + Write>(
    stream: S,
    hostname: &str,
    config: &TlsConfig,
) -> Result<RecordLayer<S>, TlsError> {
    let mut state = Handshake::new(stream, hostname, config)?;

    match state.run() {
        Ok(()) => Ok(state.layer),
        Err(e) => {
            // Tell the peer why, so it does not sit waiting on a socket that
            // will never carry another byte.
            state.layer.send_alert(alert_for(&e));
            Err(e)
        }
    }
}

fn alert_for(e: &TlsError) -> u8 {
    match e {
        TlsError::Decode(_) => alert::DECODE_ERROR,
        TlsError::Crypto(_) => alert::BAD_RECORD_MAC,
        TlsError::BadCertificate(_) => alert::BAD_CERTIFICATE,
        TlsError::Unsupported(_) => alert::HANDSHAKE_FAILURE,
        TlsError::UnexpectedMessage => alert::UNEXPECTED_MESSAGE,
        _ => alert::HANDSHAKE_FAILURE,
    }
}

struct Handshake<'a, S: Read + Write> {
    layer: RecordLayer<S>,
    hostname: String,
    config: &'a TlsConfig,
    transcript: Transcript,
    reader: HandshakeReader,
    private_key: Option<EphemeralPrivateKey>,
    client_public: Vec<u8>,
    suite: Option<CipherSuite>,
    schedule: Option<KeySchedule>,
    client_handshake_secret: Vec<u8>,
    server_handshake_secret: Vec<u8>,
    chain: Vec<Certificate>,
}

impl<'a, S: Read + Write> Handshake<'a, S> {
    fn new(stream: S, hostname: &str, config: &'a TlsConfig) -> Result<Self, TlsError> {
        let rng = SystemRandom::new();
        let private_key = EphemeralPrivateKey::generate(&X25519, &rng)
            .map_err(|_| TlsError::Crypto("key generation failed"))?;
        let client_public = private_key
            .compute_public_key()
            .map_err(|_| TlsError::Crypto("public key derivation failed"))?
            .as_ref()
            .to_vec();

        Ok(Self {
            layer: RecordLayer::new(stream),
            hostname: hostname.to_string(),
            config,
            transcript: Transcript::new(),
            reader: HandshakeReader::new(),
            private_key: Some(private_key),
            client_public,
            suite: None,
            schedule: None,
            client_handshake_secret: Vec::new(),
            server_handshake_secret: Vec::new(),
            chain: Vec::new(),
        })
    }

    fn run(&mut self) -> Result<(), TlsError> {
        self.send_client_hello()?;
        self.read_server_hello()?;
        self.read_encrypted_handshake()?;

        // Application secrets are derived over the transcript ending at the
        // *server's* Finished, so the hash is taken before our own Finished
        // is appended to it. That message still has to go out under the
        // handshake keys, hence the snapshot rather than a reorder.
        let transcript = self.transcript.hash();
        self.send_finished()?;
        self.install_application_keys(&transcript)?;
        Ok(())
    }

    fn send_client_hello(&mut self) -> Result<(), TlsError> {
        let rng = SystemRandom::new();
        let mut random = [0u8; 32];
        rng.fill(&mut random)
            .map_err(|_| TlsError::Crypto("rng failed"))?;

        // A non-empty legacy session id makes the handshake look like a
        // resumption attempt to middleboxes, which is what the spec asks for
        // in compatibility mode.
        let mut session_id = [0u8; 32];
        rng.fill(&mut session_id)
            .map_err(|_| TlsError::Crypto("rng failed"))?;

        let mut body = Writer::new();
        // legacy_version is frozen at TLS 1.2; the real version is in the
        // supported_versions extension.
        body.u16(VERSION_TLS12);
        body.bytes(&random);
        body.vec8(|w| w.bytes(&session_id));

        body.vec16(|w| {
            w.u16(CipherSuite::Aes128GcmSha256.as_u16());
            w.u16(CipherSuite::ChaCha20Poly1305Sha256.as_u16());
        });

        // legacy_compression_methods: a single "null" entry.
        body.vec8(|w| w.u8(0));

        let hostname = self.hostname.clone();
        let alpn = self.config.alpn.clone();
        let client_public = self.client_public.clone();

        body.vec16(|w| {
            // server_name: skipped for IP literals, where SNI is not allowed.
            if !is_ip_literal(&hostname) {
                w.u16(ext::SERVER_NAME);
                w.vec16(|w| {
                    w.vec16(|w| {
                        w.u8(0); // host_name
                        w.vec16(|w| w.bytes(hostname.as_bytes()));
                    });
                });
            }

            w.u16(ext::SUPPORTED_VERSIONS);
            w.vec16(|w| w.vec8(|w| w.u16(VERSION_TLS13)));

            w.u16(ext::SUPPORTED_GROUPS);
            w.vec16(|w| w.vec16(|w| w.u16(GROUP_X25519)));

            w.u16(ext::SIGNATURE_ALGORITHMS);
            w.vec16(|w| {
                w.vec16(|w| {
                    for &s in sig::ALL {
                        w.u16(s);
                    }
                })
            });

            // The key share is sent up front so a normal handshake needs
            // only one round trip.
            w.u16(ext::KEY_SHARE);
            w.vec16(|w| {
                w.vec16(|w| {
                    w.u16(GROUP_X25519);
                    w.vec16(|w| w.bytes(&client_public));
                })
            });

            if !alpn.is_empty() {
                w.u16(ext::ALPN);
                w.vec16(|w| {
                    w.vec16(|w| {
                        for proto in &alpn {
                            w.vec8(|w| w.bytes(proto.as_bytes()));
                        }
                    })
                });
            }
        });

        self.write_handshake(HandshakeType::ClientHello, &body.into_vec())
    }

    fn write_handshake(&mut self, typ: HandshakeType, body: &[u8]) -> Result<(), TlsError> {
        let mut msg = Writer::new();
        msg.u8(typ as u8);
        msg.vec24(|w| w.bytes(body));

        let msg = msg.into_vec();
        self.transcript.add(&msg);
        self.layer.write_record(ContentType::Handshake, &msg)
    }

    /// Reads the next handshake message, pulling records until one is whole.
    fn next_message(&mut self) -> Result<(HandshakeType, Vec<u8>), TlsError> {
        loop {
            if let Some(msg) = self.reader.next()? {
                return Ok(msg);
            }

            match self.layer.read_record()? {
                Some((ContentType::Handshake, data)) => self.reader.feed(&data),
                Some((ContentType::ApplicationData, _)) => {
                    return Err(TlsError::UnexpectedMessage)
                }
                Some(_) | None => continue,
            }
        }
    }

    fn read_server_hello(&mut self) -> Result<(), TlsError> {
        let (typ, raw) = self.next_message()?;
        if typ != HandshakeType::ServerHello {
            return Err(TlsError::UnexpectedMessage);
        }

        let mut r = Reader::new(&raw[4..]);
        let legacy_version = r.u16()?;
        let random = r.take(32)?.to_vec();
        let _session_id = r.vec8()?;
        let suite = r.u16()?;
        let _compression = r.u8()?;
        let extensions = r.vec16()?;

        if legacy_version != VERSION_TLS12 {
            return Err(TlsError::Unsupported("server sent a bad legacy version"));
        }

        // A HelloRetryRequest is a ServerHello with this exact random. It
        // asks for a different key share; since X25519 is all we offer,
        // there is nothing to retry with.
        const HELLO_RETRY_REQUEST: [u8; 32] = [
            0xCF, 0x21, 0xAD, 0x74, 0xE5, 0x9A, 0x61, 0x11, 0xBE, 0x1D, 0x8C, 0x02, 0x1E, 0x65,
            0xB8, 0x91, 0xC2, 0xA2, 0x11, 0x16, 0x7A, 0xBB, 0x8C, 0x5E, 0x07, 0x9E, 0x09, 0xE2,
            0xC8, 0xA8, 0x33, 0x9C,
        ];
        if random[..] == HELLO_RETRY_REQUEST {
            return Err(TlsError::Unsupported(
                "server asked for a different key share group",
            ));
        }

        let suite = CipherSuite::from_u16(suite)
            .ok_or(TlsError::Unsupported("server chose a cipher suite we did not offer"))?;

        let (version, server_public) = parse_server_hello_extensions(extensions)?;
        if version != VERSION_TLS13 {
            return Err(TlsError::Unsupported("server did not select TLS 1.3"));
        }

        self.transcript.add(&raw);
        self.suite = Some(suite);

        // ECDHE: our private key plus their public one yields the shared
        // secret that every later key is derived from.
        let private_key = self
            .private_key
            .take()
            .ok_or(TlsError::Crypto("key already consumed"))?;
        let peer = UnparsedPublicKey::new(&X25519, server_public);

        let shared = agreement::agree_ephemeral(private_key, &peer, |material| material.to_vec())
            .map_err(|_| TlsError::Crypto("key agreement failed"))?;

        let mut schedule = KeySchedule::new(suite);
        schedule.enter_handshake(&shared)?;

        let transcript = self.transcript.hash();
        let client = schedule.traffic_keys(b"c hs traffic", &transcript)?;
        let server = schedule.traffic_keys(b"s hs traffic", &transcript)?;

        self.client_handshake_secret = client.secret.clone();
        self.server_handshake_secret = server.secret.clone();

        self.layer
            .set_write_key(RecordCrypto::new(suite, &client.key, &client.iv)?);
        self.layer
            .set_read_key(RecordCrypto::new(suite, &server.key, &server.iv)?);

        self.schedule = Some(schedule);
        Ok(())
    }

    /// Reads everything from EncryptedExtensions through the server's
    /// Finished, verifying the certificate and both proofs.
    fn read_encrypted_handshake(&mut self) -> Result<(), TlsError> {
        let mut seen_encrypted_extensions = false;
        let mut cert_verify_transcript = None;

        loop {
            let (typ, raw) = self.next_message()?;

            match typ {
                HandshakeType::EncryptedExtensions => {
                    if seen_encrypted_extensions {
                        return Err(TlsError::UnexpectedMessage);
                    }
                    seen_encrypted_extensions = true;
                    self.transcript.add(&raw);
                }

                HandshakeType::CertificateRequest => {
                    // Client certificates are not implemented. Continuing
                    // without one is legal; the server decides whether that
                    // is acceptable.
                    self.transcript.add(&raw);
                }

                HandshakeType::Certificate => {
                    if !seen_encrypted_extensions {
                        return Err(TlsError::UnexpectedMessage);
                    }
                    self.chain = parse_certificate(&raw[4..])?;
                    // The hash *before* CertificateVerify is what the
                    // signature covers, so snapshot it after adding this
                    // message but before the next.
                    self.transcript.add(&raw);
                    cert_verify_transcript = Some(self.transcript.hash());

                    self.config
                        .verifier
                        .verify(&self.chain, &self.hostname, now_unix())?;
                }

                HandshakeType::CertificateVerify => {
                    let signed = cert_verify_transcript
                        .take()
                        .ok_or(TlsError::UnexpectedMessage)?;
                    let leaf = self
                        .chain
                        .first()
                        .ok_or(TlsError::BadCertificate("no certificate to verify against"))?;

                    verify_certificate_verify(&raw[4..], leaf, &signed)?;
                    self.transcript.add(&raw);
                }

                HandshakeType::Finished => {
                    // The server proves it derived the same handshake secret
                    // over the same transcript. Nothing before this point is
                    // trustworthy without it.
                    let transcript = self.transcript.hash();
                    let expected = finished_mac(&self.server_handshake_secret, &transcript)?;
                    let got = &raw[4..];

                    if !constant_time_eq(&expected, got) {
                        return Err(TlsError::Crypto("server Finished did not verify"));
                    }

                    if self.chain.is_empty() {
                        return Err(TlsError::BadCertificate(
                            "server authenticated without a certificate",
                        ));
                    }

                    self.transcript.add(&raw);
                    return Ok(());
                }

                _ => return Err(TlsError::UnexpectedMessage),
            }
        }
    }

    fn send_finished(&mut self) -> Result<(), TlsError> {
        let transcript = self.transcript.hash();
        let mac = finished_mac(&self.client_handshake_secret, &transcript)?;
        self.write_handshake(HandshakeType::Finished, &mac)
    }

    fn install_application_keys(&mut self, transcript: &[u8]) -> Result<(), TlsError> {
        let suite = self.suite.ok_or(TlsError::Crypto("no cipher suite"))?;
        let schedule = self
            .schedule
            .as_mut()
            .ok_or(TlsError::Crypto("no key schedule"))?;

        schedule.enter_application()?;

        let client = schedule.traffic_keys(b"c ap traffic", transcript)?;
        let server = schedule.traffic_keys(b"s ap traffic", transcript)?;

        self.layer
            .set_write_key(RecordCrypto::new(suite, &client.key, &client.iv)?);
        self.layer
            .set_read_key(RecordCrypto::new(suite, &server.key, &server.iv)?);

        Ok(())
    }
}

/// Pulls the selected version and key share out of a ServerHello.
fn parse_server_hello_extensions(input: &[u8]) -> Result<(u16, &[u8]), TlsError> {
    let mut r = Reader::new(input);
    let mut version = None;
    let mut key_share = None;

    while !r.is_empty() {
        let typ = r.u16()?;
        let body = r.vec16()?;

        match typ {
            ext::SUPPORTED_VERSIONS => {
                let mut b = Reader::new(body);
                version = Some(b.u16()?);
            }
            ext::KEY_SHARE => {
                let mut b = Reader::new(body);
                let group = b.u16()?;
                if group != GROUP_X25519 {
                    return Err(TlsError::Unsupported("server chose an unoffered group"));
                }
                let share = b.vec16()?;
                if share.len() != 32 {
                    return Err(TlsError::Decode("bad x25519 key share length"));
                }
                key_share = Some(share);
            }
            _ => {}
        }
    }

    match (version, key_share) {
        (Some(v), Some(k)) => Ok((v, k)),
        (None, _) => Err(TlsError::Unsupported("server omitted supported_versions")),
        (_, None) => Err(TlsError::Decode("server omitted its key share")),
    }
}

/// Parses a Certificate message into its chain.
fn parse_certificate(input: &[u8]) -> Result<Vec<Certificate>, TlsError> {
    let mut r = Reader::new(input);

    let context = r.vec8()?;
    if !context.is_empty() {
        return Err(TlsError::Decode("unexpected certificate request context"));
    }

    let list = r.vec24()?;
    let mut entries = Reader::new(list);
    let mut chain = Vec::new();

    while !entries.is_empty() {
        let der = entries.vec24()?;
        let _extensions = entries.vec16()?;

        // Only the leaf is parsed. The rest of the chain is not used: this
        // client pins, and path building is not implemented.
        if chain.is_empty() {
            chain.push(Certificate::parse(der)?);
        } else {
            chain.push(Certificate {
                der: der.to_vec(),
                ..chain[0].clone()
            });
        }
    }

    if chain.is_empty() {
        return Err(TlsError::BadCertificate("server sent an empty chain"));
    }

    Ok(chain)
}

/// Checks the CertificateVerify signature.
///
/// This is what proves the server holds the private key for the certificate
/// it presented, rather than having copied someone else's certificate.
fn verify_certificate_verify(
    input: &[u8],
    leaf: &Certificate,
    transcript_hash: &[u8],
) -> Result<(), TlsError> {
    let mut r = Reader::new(input);
    let scheme = r.u16()?;
    let signature = r.vec16()?;
    r.expect_empty()?;

    // The signed content is a fixed prefix, a context string, a separator,
    // and the transcript hash. The prefix exists so a signature can never be
    // confused with one made for a different purpose.
    let mut signed = Vec::with_capacity(64 + 34 + transcript_hash.len());
    signed.extend_from_slice(&[0x20; 64]);
    signed.extend_from_slice(SERVER_CERT_VERIFY_CONTEXT);
    signed.push(0);
    signed.extend_from_slice(transcript_hash);

    let algorithm: &dyn ring::signature::VerificationAlgorithm = match scheme {
        sig::ECDSA_NISTP256_SHA256 => &ring::signature::ECDSA_P256_SHA256_ASN1,
        sig::ECDSA_NISTP384_SHA384 => &ring::signature::ECDSA_P384_SHA384_ASN1,
        sig::RSA_PSS_SHA256 => &ring::signature::RSA_PSS_2048_8192_SHA256,
        sig::RSA_PSS_SHA384 => &ring::signature::RSA_PSS_2048_8192_SHA384,
        sig::RSA_PSS_SHA512 => &ring::signature::RSA_PSS_2048_8192_SHA512,
        sig::RSA_PKCS1_SHA256 => &ring::signature::RSA_PKCS1_2048_8192_SHA256,
        sig::RSA_PKCS1_SHA384 => &ring::signature::RSA_PKCS1_2048_8192_SHA384,
        sig::RSA_PKCS1_SHA512 => &ring::signature::RSA_PKCS1_2048_8192_SHA512,
        _ => return Err(TlsError::Unsupported("server used an unoffered signature scheme")),
    };

    ring::signature::UnparsedPublicKey::new(algorithm, &leaf.public_key)
        .verify(&signed, signature)
        .map_err(|_| TlsError::BadCertificate("CertificateVerify signature is invalid"))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Whether a hostname is an IP literal, which must not appear in SNI.
fn is_ip_literal(host: &str) -> bool {
    host.parse::<std::net::IpAddr>().is_ok()
}
