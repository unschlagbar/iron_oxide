//! The TLS record layer: framing, and AEAD protection once keys exist.
//!
//! A record is a 5-byte header (type, legacy version, length) followed by a
//! fragment. Before the handshake produces keys the fragment is plaintext;
//! afterwards it is an AEAD ciphertext whose real content type is the last
//! non-zero byte of the decrypted plaintext, with the outer type always
//! claiming `application_data`.

use std::io::{Read, Write};

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey};

use super::TlsError;
use super::codec::Reader;
use super::msgs::{CipherSuite, ContentType, MAX_FRAGMENT, VERSION_TLS12};

/// One direction's AEAD state: a key plus the sequence number that makes
/// every nonce unique.
pub struct RecordCrypto {
    key: LessSafeKey,
    iv: [u8; 12],
    seq: u64,
}

impl RecordCrypto {
    pub fn new(suite: CipherSuite, key: &[u8], iv: &[u8]) -> Result<Self, TlsError> {
        let unbound =
            UnboundKey::new(suite.aead(), key).map_err(|_| TlsError::Crypto("bad aead key"))?;
        let mut iv_buf = [0u8; 12];
        if iv.len() != 12 {
            return Err(TlsError::Crypto("bad iv length"));
        }
        iv_buf.copy_from_slice(iv);

        Ok(Self {
            key: LessSafeKey::new(unbound),
            iv: iv_buf,
            seq: 0,
        })
    }

    /// The per-record nonce: the static IV xored with the sequence number,
    /// right-aligned. Reusing a nonce under the same key breaks the AEAD
    /// completely, so the counter is stepped on every single record and the
    /// caller is never given a say in it.
    fn next_nonce(&mut self) -> Result<Nonce, TlsError> {
        let mut nonce = self.iv;
        let seq = self.seq.to_be_bytes();
        for i in 0..8 {
            nonce[4 + i] ^= seq[i];
        }

        // Wrapping the sequence number would repeat a nonce. TLS requires
        // a key update long before this, so refuse rather than wrap.
        self.seq = self
            .seq
            .checked_add(1)
            .ok_or(TlsError::Crypto("record sequence exhausted"))?;

        Ok(Nonce::assume_unique_for_key(nonce))
    }

    /// Encrypts a fragment, producing the inner plaintext layout TLS 1.3
    /// uses: content ‖ real_type, then the tag.
    fn seal(&mut self, typ: ContentType, payload: &[u8]) -> Result<Vec<u8>, TlsError> {
        let mut inner = Vec::with_capacity(payload.len() + 1 + 16);
        inner.extend_from_slice(payload);
        inner.push(typ as u8);

        // The AAD is the record header the ciphertext will ship with, so a
        // tampered length or type fails the tag check.
        let len = (inner.len() + self.key.algorithm().tag_len()) as u16;
        let aad_bytes = [
            ContentType::ApplicationData as u8,
            (VERSION_TLS12 >> 8) as u8,
            VERSION_TLS12 as u8,
            (len >> 8) as u8,
            len as u8,
        ];

        let nonce = self.next_nonce()?;
        self.key
            .seal_in_place_append_tag(nonce, Aad::from(aad_bytes), &mut inner)
            .map_err(|_| TlsError::Crypto("seal failed"))?;

        Ok(inner)
    }

    /// Decrypts a fragment and recovers its true content type.
    fn open(
        &mut self,
        header: [u8; 5],
        mut body: Vec<u8>,
    ) -> Result<(ContentType, Vec<u8>), TlsError> {
        let nonce = self.next_nonce()?;
        let plain_len = self
            .key
            .open_in_place(nonce, Aad::from(header), &mut body)
            .map_err(|_| TlsError::Crypto("decryption failed"))?
            .len();
        body.truncate(plain_len);

        // Strip zero padding to find the real content type.
        while body.last() == Some(&0) {
            body.pop();
        }
        let typ = body
            .pop()
            .and_then(ContentType::from_u8)
            .ok_or(TlsError::Decode("bad inner content type"))?;

        Ok((typ, body))
    }
}

/// Reads and writes records over a byte stream, encrypting once keys are
/// installed.
pub struct RecordLayer<S> {
    pub stream: S,
    write_crypto: Option<RecordCrypto>,
    read_crypto: Option<RecordCrypto>,
}

impl<S: Read + Write> RecordLayer<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            write_crypto: None,
            read_crypto: None,
        }
    }

    /// Installs write keys. Subsequent records are encrypted.
    pub fn set_write_key(&mut self, crypto: RecordCrypto) {
        self.write_crypto = Some(crypto);
    }

    /// Installs read keys. Subsequent records are expected encrypted.
    pub fn set_read_key(&mut self, crypto: RecordCrypto) {
        self.read_crypto = Some(crypto);
    }

    /// Writes one record, fragmenting if the payload exceeds the limit.
    pub fn write_record(&mut self, typ: ContentType, payload: &[u8]) -> Result<(), TlsError> {
        match &mut self.write_crypto {
            Some(crypto) => write_encrypted(&mut self.stream, crypto, typ, payload),
            None => {
                // Before keys exist the fragment goes out as-is, with its
                // real content type on the wire.
                for chunk in fragments(payload) {
                    let mut out = Vec::with_capacity(chunk.len() + 5);
                    out.push(typ as u8);
                    out.extend_from_slice(&VERSION_TLS12.to_be_bytes());
                    out.extend_from_slice(&(chunk.len() as u16).to_be_bytes());
                    out.extend_from_slice(chunk);
                    self.stream.write_all(&out)?;
                }
                self.stream.flush()?;
                Ok(())
            }
        }
    }

    /// Reads one record, decrypting if keys are installed.
    ///
    /// Returns `None` for records that carry nothing the caller should see,
    /// namely the compatibility ChangeCipherSpec.
    pub fn read_record(&mut self) -> Result<Option<(ContentType, Vec<u8>)>, TlsError> {
        read_record_from(&mut self.stream, self.read_crypto.as_mut())
    }

    /// Splits into a read layer and the write crypto, for use from separate
    /// threads.
    ///
    /// Only valid once the handshake has installed both keys; the two
    /// directions share no state after that.
    pub fn into_halves(self) -> (ReadLayer<S>, RecordCrypto) {
        (
            ReadLayer {
                stream: self.stream,
                crypto: self.read_crypto,
            },
            self.write_crypto
                .expect("split requires a completed handshake"),
        )
    }

    /// Sends a fatal alert, best-effort. Used on the way out of a failed
    /// handshake so the peer learns why.
    pub fn send_alert(&mut self, description: u8) {
        let _ = self.write_record(
            ContentType::Alert,
            &[super::msgs::alert::LEVEL_FATAL, description],
        );
    }
}

/// The read side of a split session.
pub struct ReadLayer<S> {
    stream: S,
    crypto: Option<RecordCrypto>,
}

impl<S: Read> ReadLayer<S> {
    pub fn read_record(&mut self) -> Result<Option<(ContentType, Vec<u8>)>, TlsError> {
        read_record_from(&mut self.stream, self.crypto.as_mut())
    }
}

/// Writes one record, encrypting it. Shared by the full layer and the write
/// half so the framing exists in exactly one place.
pub fn write_encrypted(
    stream: &mut impl Write,
    crypto: &mut RecordCrypto,
    typ: ContentType,
    payload: &[u8],
) -> Result<(), TlsError> {
    for chunk in fragments(payload) {
        let body = crypto.seal(typ, chunk)?;
        let mut out = Vec::with_capacity(body.len() + 5);
        out.push(ContentType::ApplicationData as u8);
        out.extend_from_slice(&VERSION_TLS12.to_be_bytes());
        out.extend_from_slice(&(body.len() as u16).to_be_bytes());
        out.extend_from_slice(&body);
        stream.write_all(&out)?;
    }
    stream.flush()?;
    Ok(())
}

/// Splits a payload into record-sized pieces, yielding one empty piece for an
/// empty payload so a zero-length write still produces a record.
fn fragments(payload: &[u8]) -> impl Iterator<Item = &[u8]> {
    payload.chunks(MAX_FRAGMENT).chain(if payload.is_empty() {
        Some(&[][..])
    } else {
        None
    })
}

/// Reads and optionally decrypts one record.
fn read_record_from(
    stream: &mut impl Read,
    crypto: Option<&mut RecordCrypto>,
) -> Result<Option<(ContentType, Vec<u8>)>, TlsError> {
    let mut header = [0u8; 5];
    stream.read_exact(&mut header)?;

    let typ = ContentType::from_u8(header[0]).ok_or(TlsError::Decode("bad content type"))?;
    let len = u16::from_be_bytes([header[3], header[4]]) as usize;

    if len > MAX_FRAGMENT + 256 {
        return Err(TlsError::Decode("record too large"));
    }

    let mut body = vec![0u8; len];
    stream.read_exact(&mut body)?;

    if typ == ContentType::ChangeCipherSpec {
        return Ok(None);
    }

    let (typ, body) = match crypto {
        Some(crypto) => crypto.open(header, body)?,
        None => (typ, body),
    };

    if typ == ContentType::Alert {
        return Err(decode_alert(&body));
    }

    Ok(Some((typ, body)))
}

fn decode_alert(body: &[u8]) -> TlsError {
    let mut r = Reader::new(body);
    match (r.u8(), r.u8()) {
        (Ok(_level), Ok(desc)) => {
            if desc == super::msgs::alert::CLOSE_NOTIFY {
                TlsError::Closed
            } else {
                TlsError::Alert(desc)
            }
        }
        _ => TlsError::Decode("malformed alert"),
    }
}
