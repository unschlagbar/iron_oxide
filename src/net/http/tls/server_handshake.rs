//! The TLS 1.3 server handshake (RFC 8446 §4), the mirror of `handshake.rs`.
//!
//! ```text
//! <- ClientHello
//! -> ServerHello          (our key share, chosen suite)
//!    ... handshake keys from here on
//! -> EncryptedExtensions
//! -> Certificate          (our chain)
//! -> CertificateVerify    (signature proving we hold the private key)
//! -> Finished
//! <- Finished
//!    ... application keys
//! ```
//!
//! Client certificates are not requested, so there is no verification of the
//! peer: any client may connect, which is what a public web server wants.

use std::io::{Read, Write};

use ring::agreement::{self, EphemeralPrivateKey, UnparsedPublicKey, X25519};
use ring::digest::{Context, SHA256};
use ring::rand::{SecureRandom, SystemRandom};
use ring::signature::{EcdsaKeyPair, RsaKeyPair};

use super::codec::{Reader, Writer};
use super::key_schedule::{KeySchedule, finished_mac};
use super::msgs::{
    CipherSuite, ContentType, GROUP_X25519, HandshakeType, SERVER_CERT_VERIFY_CONTEXT,
    VERSION_TLS12, VERSION_TLS13, alert, ext, sig,
};
use super::record::{RecordCrypto, RecordLayer};
use super::{ServerTlsConfig, TlsError};

/// The private key behind the server's certificate.
pub enum SigningKey {
    Rsa(Box<RsaKeyPair>),
    EcdsaP256(Box<EcdsaKeyPair>),
}

impl SigningKey {
    /// Loads a PKCS#8 key, detecting which kind it is.
    pub fn from_pkcs8(der: &[u8]) -> Result<Self, TlsError> {
        if let Ok(rsa) = RsaKeyPair::from_pkcs8(der) {
            return Ok(Self::Rsa(Box::new(rsa)));
        }

        let rng = SystemRandom::new();
        if let Ok(ec) = EcdsaKeyPair::from_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
            der,
            &rng,
        ) {
            return Ok(Self::EcdsaP256(Box::new(ec)));
        }

        Err(TlsError::Unsupported(
            "private key must be PKCS#8 RSA or ECDSA P-256",
        ))
    }

    /// The signature scheme this key will use in CertificateVerify.
    fn scheme(&self) -> u16 {
        match self {
            Self::Rsa(_) => sig::RSA_PSS_SHA256,
            Self::EcdsaP256(_) => sig::ECDSA_NISTP256_SHA256,
        }
    }

    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, TlsError> {
        let rng = SystemRandom::new();
        match self {
            Self::Rsa(key) => {
                let mut out = vec![0u8; key.public().modulus_len()];
                key.sign(&ring::signature::RSA_PSS_SHA256, &rng, message, &mut out)
                    .map_err(|_| TlsError::Crypto("rsa signing failed"))?;
                Ok(out)
            }
            Self::EcdsaP256(key) => key
                .sign(&rng, message)
                .map(|s| s.as_ref().to_vec())
                .map_err(|_| TlsError::Crypto("ecdsa signing failed")),
        }
    }
}

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

    fn hash(&self) -> Vec<u8> {
        self.ctx.clone().finish().as_ref().to_vec()
    }
}

/// Runs the server side of the handshake.
pub fn accept<S: Read + Write>(
    stream: S,
    config: &ServerTlsConfig,
) -> Result<RecordLayer<S>, TlsError> {
    let mut layer = RecordLayer::new(stream);

    match run(&mut layer, config) {
        Ok(()) => Ok(layer),
        Err(e) => {
            layer.send_alert(alert_for(&e));
            Err(e)
        }
    }
}

fn alert_for(e: &TlsError) -> u8 {
    match e {
        TlsError::Decode(_) => alert::DECODE_ERROR,
        TlsError::Crypto(_) => alert::BAD_RECORD_MAC,
        TlsError::Unsupported(_) => alert::HANDSHAKE_FAILURE,
        TlsError::UnexpectedMessage => alert::UNEXPECTED_MESSAGE,
        _ => alert::HANDSHAKE_FAILURE,
    }
}

fn run<S: Read + Write>(
    layer: &mut RecordLayer<S>,
    config: &ServerTlsConfig,
) -> Result<(), TlsError> {
    let mut transcript = Transcript::new();

    // --- ClientHello ---
    let hello = read_handshake(layer, HandshakeType::ClientHello)?;
    transcript.add(&hello);
    let client = parse_client_hello(&hello[4..])?;

    let suite = client
        .suites
        .iter()
        .find_map(|&s| CipherSuite::from_u16(s))
        .ok_or(TlsError::Unsupported("no mutually supported cipher suite"))?;

    // --- ServerHello ---
    let rng = SystemRandom::new();
    let private_key = EphemeralPrivateKey::generate(&X25519, &rng)
        .map_err(|_| TlsError::Crypto("key generation failed"))?;
    let public_key = private_key
        .compute_public_key()
        .map_err(|_| TlsError::Crypto("public key derivation failed"))?;

    let mut random = [0u8; 32];
    rng.fill(&mut random)
        .map_err(|_| TlsError::Crypto("rng failed"))?;

    let mut body = Writer::new();
    body.u16(VERSION_TLS12);
    body.bytes(&random);
    // The client's legacy session id is echoed back verbatim, which is what
    // keeps middleboxes believing this is a resumed TLS 1.2 session.
    body.vec8(|w| w.bytes(&client.session_id));
    body.u16(suite.as_u16());
    body.u8(0);
    body.vec16(|w| {
        w.u16(ext::SUPPORTED_VERSIONS);
        w.vec16(|w| w.u16(VERSION_TLS13));

        w.u16(ext::KEY_SHARE);
        w.vec16(|w| {
            w.u16(GROUP_X25519);
            w.vec16(|w| w.bytes(public_key.as_ref()));
        });
    });

    let server_hello = frame(HandshakeType::ServerHello, &body.into_vec());
    transcript.add(&server_hello);
    layer.write_record(ContentType::Handshake, &server_hello)?;

    // --- handshake keys ---
    let peer = UnparsedPublicKey::new(&X25519, &client.key_share);
    let shared = agreement::agree_ephemeral(private_key, &peer, |m| m.to_vec())
        .map_err(|_| TlsError::Crypto("key agreement failed"))?;

    let mut schedule = KeySchedule::new(suite);
    schedule.enter_handshake(&shared)?;

    let hs_hash = transcript.hash();
    let client_hs = schedule.traffic_keys(b"c hs traffic", &hs_hash)?;
    let server_hs = schedule.traffic_keys(b"s hs traffic", &hs_hash)?;

    layer.set_write_key(RecordCrypto::new(suite, &server_hs.key, &server_hs.iv)?);
    layer.set_read_key(RecordCrypto::new(suite, &client_hs.key, &client_hs.iv)?);

    // --- EncryptedExtensions ---
    let mut ee = Writer::new();
    ee.vec16(|w| {
        // ALPN is echoed only if the client offered something we serve.
        if let Some(proto) = client
            .alpn
            .iter()
            .find(|p| config.alpn.iter().any(|ours| ours == *p))
        {
            w.u16(ext::ALPN);
            w.vec16(|w| w.vec16(|w| w.vec8(|w| w.bytes(proto.as_bytes()))));
        }
    });
    let ee = frame(HandshakeType::EncryptedExtensions, &ee.into_vec());
    transcript.add(&ee);
    layer.write_record(ContentType::Handshake, &ee)?;

    // --- Certificate ---
    let mut cert = Writer::new();
    cert.vec8(|_| {}); // empty certificate_request_context
    cert.vec24(|w| {
        for der in &config.chain {
            w.vec24(|w| w.bytes(der));
            w.vec16(|_| {}); // no per-certificate extensions
        }
    });
    let cert = frame(HandshakeType::Certificate, &cert.into_vec());
    transcript.add(&cert);
    layer.write_record(ContentType::Handshake, &cert)?;

    // --- CertificateVerify ---
    // Signing the transcript is what proves we hold the private key for the
    // certificate just sent, rather than having copied someone else's.
    let mut signed = Vec::new();
    signed.extend_from_slice(&[0x20; 64]);
    signed.extend_from_slice(SERVER_CERT_VERIFY_CONTEXT);
    signed.push(0);
    signed.extend_from_slice(&transcript.hash());

    let signature = config.key.sign(&signed)?;
    let mut cv = Writer::new();
    cv.u16(config.key.scheme());
    cv.vec16(|w| w.bytes(&signature));
    let cv = frame(HandshakeType::CertificateVerify, &cv.into_vec());
    transcript.add(&cv);
    layer.write_record(ContentType::Handshake, &cv)?;

    // --- our Finished ---
    let mac = finished_mac(&server_hs.secret, &transcript.hash())?;
    let fin = frame(HandshakeType::Finished, &mac);
    transcript.add(&fin);
    layer.write_record(ContentType::Handshake, &fin)?;

    // Application keys cover the transcript through our Finished, before the
    // client's is added.
    let app_hash = transcript.hash();

    // --- client Finished ---
    let client_fin = read_handshake(layer, HandshakeType::Finished)?;
    let expected = finished_mac(&client_hs.secret, &transcript.hash())?;
    if !constant_time_eq(&expected, &client_fin[4..]) {
        return Err(TlsError::Crypto("client Finished did not verify"));
    }

    // --- application keys ---
    schedule.enter_application()?;
    let client_app = schedule.traffic_keys(b"c ap traffic", &app_hash)?;
    let server_app = schedule.traffic_keys(b"s ap traffic", &app_hash)?;

    layer.set_write_key(RecordCrypto::new(suite, &server_app.key, &server_app.iv)?);
    layer.set_read_key(RecordCrypto::new(suite, &client_app.key, &client_app.iv)?);

    Ok(())
}

/// What the server needs out of a ClientHello.
struct ClientHello {
    session_id: Vec<u8>,
    suites: Vec<u16>,
    key_share: Vec<u8>,
    alpn: Vec<String>,
}

fn parse_client_hello(input: &[u8]) -> Result<ClientHello, TlsError> {
    let mut r = Reader::new(input);

    let _legacy_version = r.u16()?;
    let _random = r.take(32)?;
    let session_id = r.vec8()?.to_vec();

    let suite_bytes = r.vec16()?;
    let mut suites = Vec::with_capacity(suite_bytes.len() / 2);
    let mut sr = Reader::new(suite_bytes);
    while !sr.is_empty() {
        suites.push(sr.u16()?);
    }

    let _compression = r.vec8()?;
    let extensions = r.vec16()?;

    let mut key_share = None;
    let mut alpn = Vec::new();
    let mut offers_tls13 = false;

    let mut er = Reader::new(extensions);
    while !er.is_empty() {
        let typ = er.u16()?;
        let body = er.vec16()?;

        match typ {
            ext::SUPPORTED_VERSIONS => {
                let mut b = Reader::new(body);
                let list = b.vec8()?;
                let mut lr = Reader::new(list);
                while !lr.is_empty() {
                    if lr.u16()? == VERSION_TLS13 {
                        offers_tls13 = true;
                    }
                }
            }
            ext::KEY_SHARE => {
                let mut b = Reader::new(body);
                let shares = b.vec16()?;
                let mut sr = Reader::new(shares);
                while !sr.is_empty() {
                    let group = sr.u16()?;
                    let share = sr.vec16()?;
                    if group == GROUP_X25519 && share.len() == 32 {
                        key_share = Some(share.to_vec());
                    }
                }
            }
            ext::ALPN => {
                let mut b = Reader::new(body);
                let list = b.vec16()?;
                let mut lr = Reader::new(list);
                while !lr.is_empty() {
                    let proto = lr.vec8()?;
                    if let Ok(s) = std::str::from_utf8(proto) {
                        alpn.push(s.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    if !offers_tls13 {
        return Err(TlsError::Unsupported("client does not support TLS 1.3"));
    }

    Ok(ClientHello {
        session_id,
        suites,
        // Without an X25519 share there is nothing to agree on. A real
        // server would answer HelloRetryRequest; we only offer one group.
        key_share: key_share
            .ok_or(TlsError::Unsupported("client sent no X25519 key share"))?,
        alpn,
    })
}

/// Reads one handshake message of the expected type, reassembling across
/// records.
fn read_handshake<S: Read + Write>(
    layer: &mut RecordLayer<S>,
    expected: HandshakeType,
) -> Result<Vec<u8>, TlsError> {
    let mut buf: Vec<u8> = Vec::new();

    loop {
        if buf.len() >= 4 {
            let len = u32::from_be_bytes([0, buf[1], buf[2], buf[3]]) as usize;
            if buf.len() >= 4 + len {
                buf.truncate(4 + len);
                let typ = HandshakeType::from_u8(buf[0])
                    .ok_or(TlsError::Decode("unknown handshake type"))?;
                if typ != expected {
                    return Err(TlsError::UnexpectedMessage);
                }
                return Ok(buf);
            }
        }

        match layer.read_record()? {
            Some((ContentType::Handshake, data)) => buf.extend_from_slice(&data),
            Some((ContentType::ApplicationData, _)) => return Err(TlsError::UnexpectedMessage),
            Some(_) | None => continue,
        }
    }
}

/// Wraps a body in its handshake header.
fn frame(typ: HandshakeType, body: &[u8]) -> Vec<u8> {
    let mut w = Writer::new();
    w.u8(typ as u8);
    w.vec24(|w| w.bytes(body));
    w.into_vec()
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
