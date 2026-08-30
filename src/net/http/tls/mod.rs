//! A TLS 1.3 client, written against `ring` for the primitives.
//!
//! Scope, stated plainly: TLS 1.3 only (no 1.2 fallback), X25519 key
//! exchange, AES-128-GCM or ChaCha20-Poly1305, and client-side only. There
//! is no session resumption, no early data, and no client certificates.
//!
//! What this module implements itself is the protocol: record framing, the
//! handshake state machine, the key schedule, and the certificate checks.
//! The constant-time arithmetic underneath — AEAD, X25519, signature
//! verification, HMAC — is `ring`'s, because that is the part where a subtle
//! mistake is invisible from the outside.
//!
//! ## Trust
//!
//! [`TlsConfig::verifier`] decides what counts as a valid peer. The default
//! is [`Verifier::Pinned`] with an empty set, which rejects everything: this
//! client ships no root store, so it cannot verify a public CA chain and
//! will not pretend otherwise. Point it at the specific certificate you
//! expect, or supply roots yourself.

use std::io::{self, Read, Write};

mod cert;
mod codec;
mod handshake;
mod key_schedule;
mod msgs;
pub mod pem;
mod record;
mod server_handshake;
mod verify;

pub use cert::Certificate;
pub use server_handshake::SigningKey;
pub use verify::Verifier;

use record::RecordLayer;

#[derive(Debug)]
pub enum TlsError {
    /// A message did not parse.
    Decode(&'static str),
    /// A cryptographic operation failed, including a failed tag check.
    Crypto(&'static str),
    /// The peer sent a fatal alert.
    Alert(u8),
    /// The peer closed the connection cleanly.
    Closed,
    /// The peer's certificate was not acceptable.
    BadCertificate(&'static str),
    /// The peer wanted something this client does not do.
    Unsupported(&'static str),
    /// The handshake did not follow the expected order.
    UnexpectedMessage,
    Io(io::Error),
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decode(what) => write!(f, "tls decode error: {what}"),
            Self::Crypto(what) => write!(f, "tls crypto error: {what}"),
            Self::Alert(code) => {
                write!(f, "tls alert from peer: {}", msgs::alert::describe(*code))
            }
            Self::Closed => write!(f, "tls connection closed by peer"),
            Self::BadCertificate(why) => write!(f, "tls certificate rejected: {why}"),
            Self::Unsupported(what) => write!(f, "tls unsupported: {what}"),
            Self::UnexpectedMessage => write!(f, "tls handshake message out of order"),
            Self::Io(e) => write!(f, "tls io error: {e}"),
        }
    }
}

impl std::error::Error for TlsError {}

impl From<io::Error> for TlsError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<TlsError> for io::Error {
    fn from(e: TlsError) -> Self {
        match e {
            TlsError::Io(e) => e,
            TlsError::Closed => io::Error::new(io::ErrorKind::UnexpectedEof, e.to_string()),
            other => io::Error::other(other.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// How the peer's certificate chain is judged.
    pub verifier: Verifier,
    /// Protocols offered via ALPN. Empty sends no ALPN extension.
    pub alpn: Vec<String>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            // Deliberately rejects everything until the caller says what to
            // trust. There is no root store to fall back on, and silently
            // trusting anything would be worse than failing here.
            verifier: Verifier::Pinned(Vec::new()),
            alpn: vec!["http/1.1".to_string()],
        }
    }
}

impl TlsConfig {
    /// Trusts exactly these certificates, matched by their DER bytes.
    pub fn pinned(certs: Vec<Vec<u8>>) -> Self {
        Self {
            verifier: Verifier::Pinned(certs),
            ..Self::default()
        }
    }
}

/// The identity a TLS server presents.
pub struct ServerTlsConfig {
    /// The certificate chain in DER, leaf first.
    pub chain: Vec<Vec<u8>>,
    /// The private key for the leaf certificate.
    pub key: SigningKey,
    /// Protocols this server will agree to over ALPN.
    pub alpn: Vec<String>,
}

impl ServerTlsConfig {
    /// Loads a certificate chain and key from PEM files, the format
    /// `openssl` and ACME clients produce.
    pub fn from_pem_files(
        cert_path: impl AsRef<std::path::Path>,
        key_path: impl AsRef<std::path::Path>,
    ) -> Result<Self, TlsError> {
        let cert_pem = std::fs::read(cert_path)?;
        let key_pem = std::fs::read(key_path)?;
        Self::from_pem(&cert_pem, &key_pem)
    }

    pub fn from_pem(cert_pem: &[u8], key_pem: &[u8]) -> Result<Self, TlsError> {
        let chain = pem::certificates(cert_pem)?;
        let key = SigningKey::from_pkcs8(&pem::private_key(key_pem)?)?;

        Ok(Self {
            chain,
            key,
            alpn: vec!["http/1.1".to_string()],
        })
    }
}

impl std::fmt::Debug for ServerTlsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The key is deliberately not printed.
        f.debug_struct("ServerTlsConfig")
            .field("chain", &format_args!("{} certificate(s)", self.chain.len()))
            .field("alpn", &self.alpn)
            .finish()
    }
}

/// A TLS session wrapped around a byte stream.
///
/// Implements `Read`/`Write` in terms of application data, so anything that
/// speaks to a `TcpStream` can speak to this instead.
pub struct TlsStream<S: Read + Write> {
    layer: RecordLayer<S>,
    /// Decrypted application data not yet handed to the caller.
    pending: Vec<u8>,
    pending_at: usize,
    closed: bool,
}

impl<S: Read + Write> TlsStream<S> {
    /// Runs the handshake and returns a stream ready for application data.
    pub fn connect(stream: S, hostname: &str, config: &TlsConfig) -> Result<Self, TlsError> {
        let layer = handshake::connect(stream, hostname, config)?;
        Ok(Self {
            layer,
            pending: Vec::new(),
            pending_at: 0,
            closed: false,
        })
    }

    /// Runs the server side of the handshake on an accepted connection.
    pub fn accept(stream: S, config: &ServerTlsConfig) -> Result<Self, TlsError> {
        let layer = server_handshake::accept(stream, config)?;
        Ok(Self {
            layer,
            pending: Vec::new(),
            pending_at: 0,
            closed: false,
        })
    }

    pub fn get_ref(&self) -> &S {
        &self.layer.stream
    }

    pub fn get_mut(&mut self) -> &mut S {
        &mut self.layer.stream
    }

    /// Splits the session into halves that can be used from different
    /// threads at the same time.
    ///
    /// This works because the two directions of a TLS session are
    /// independent: each has its own key, IV and sequence number, and they
    /// share nothing after the handshake. So each half can own its crypto
    /// outright and no lock is needed between them — which matters, because
    /// a shared lock would deadlock the moment a reader blocked waiting for
    /// data the writer was supposed to trigger.
    ///
    /// The transport is duplicated by `clone_stream`, since each half needs
    /// its own handle on the underlying socket.
    pub fn split(
        self,
        clone_stream: impl FnOnce(&S) -> io::Result<S>,
    ) -> io::Result<(TlsReadHalf<S>, TlsWriteHalf<S>)> {
        let write_stream = clone_stream(&self.layer.stream)?;
        let (read_layer, write_crypto) = self.layer.into_halves();

        Ok((
            TlsReadHalf {
                layer: read_layer,
                pending: self.pending,
                pending_at: self.pending_at,
                closed: self.closed,
            },
            TlsWriteHalf {
                stream: write_stream,
                crypto: write_crypto,
            },
        ))
    }

    /// Sends `close_notify`, which tells the peer the stream ended on
    /// purpose rather than being cut.
    pub fn close(&mut self) -> Result<(), TlsError> {
        if !self.closed {
            self.closed = true;
            self.layer.write_record(
                msgs::ContentType::Alert,
                &[msgs::alert::LEVEL_FATAL, msgs::alert::CLOSE_NOTIFY],
            )?;
        }
        Ok(())
    }
}

/// The read half of a split session.
pub struct TlsReadHalf<S: Read> {
    layer: record::ReadLayer<S>,
    pending: Vec<u8>,
    pending_at: usize,
    closed: bool,
}

impl<S: Read> Read for TlsReadHalf<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while self.pending_at >= self.pending.len() {
            if self.closed {
                return Ok(0);
            }

            match self.layer.read_record() {
                Ok(Some((msgs::ContentType::ApplicationData, data))) => {
                    self.pending = data;
                    self.pending_at = 0;
                }
                Ok(Some(_)) | Ok(None) => continue,
                Err(TlsError::Closed) => {
                    self.closed = true;
                    return Ok(0);
                }
                Err(e) => return Err(e.into()),
            }
        }

        let n = (self.pending.len() - self.pending_at).min(buf.len());
        buf[..n].copy_from_slice(&self.pending[self.pending_at..self.pending_at + n]);
        self.pending_at += n;
        Ok(n)
    }
}

/// The write half of a split session.
pub struct TlsWriteHalf<S: Write> {
    stream: S,
    crypto: record::RecordCrypto,
}

impl<S: Write> TlsWriteHalf<S> {
    pub fn get_ref(&self) -> &S {
        &self.stream
    }
}

impl<S: Write> Write for TlsWriteHalf<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_all(buf)?;
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        record::write_encrypted(
            &mut self.stream,
            &mut self.crypto,
            msgs::ContentType::ApplicationData,
            buf,
        )
        .map_err(Into::into)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

impl<S: Read + Write> Read for TlsStream<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Drain whatever the last record left over before decrypting more.
        while self.pending_at >= self.pending.len() {
            if self.closed {
                return Ok(0);
            }

            match self.layer.read_record() {
                Ok(Some((msgs::ContentType::ApplicationData, data))) => {
                    self.pending = data;
                    self.pending_at = 0;
                }
                // Post-handshake handshake messages (session tickets, key
                // updates) are not application data; skip them.
                Ok(Some(_)) => continue,
                Ok(None) => continue,
                Err(TlsError::Closed) => {
                    self.closed = true;
                    return Ok(0);
                }
                Err(e) => return Err(e.into()),
            }
        }

        let n = (self.pending.len() - self.pending_at).min(buf.len());
        buf[..n].copy_from_slice(&self.pending[self.pending_at..self.pending_at + n]);
        self.pending_at += n;
        Ok(n)
    }
}

impl<S: Read + Write> Write for TlsStream<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.layer
            .write_record(msgs::ContentType::ApplicationData, buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.layer.stream.flush()
    }
}
