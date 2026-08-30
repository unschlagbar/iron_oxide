//! The TLS 1.3 key schedule (RFC 8446 §7.1).
//!
//! Everything hangs off a chain of HKDF-Extract steps, each mixing in one
//! more input: no PSK, then the ECDHE shared secret, then nothing. Between
//! them, traffic secrets are derived with HKDF-Expand-Label over a running
//! hash of the handshake so far — which is what binds the keys to the exact
//! messages both sides saw. A mismatch anywhere and Finished fails.
//!
//! Secrets are held as raw bytes rather than `ring::hkdf::Prk`, because the
//! schedule has to feed one stage's output back in as the next stage's salt
//! and `Prk` cannot be read back out. HKDF is just HMAC underneath, so both
//! Extract and Expand are written directly against `ring::hmac`.

use ring::digest::{digest, SHA256};
use ring::hmac;

use super::msgs::CipherSuite;
use super::TlsError;

/// SHA-256 is the only hash these suites use, so every secret is 32 bytes.
const HASH_LEN: usize = 32;

/// HKDF-Extract (RFC 5869 §2.2).
pub fn extract(salt: &[u8], ikm: &[u8]) -> Vec<u8> {
    let key = hmac::Key::new(hmac::HMAC_SHA256, salt);
    hmac::sign(&key, ikm).as_ref().to_vec()
}

/// HKDF-Expand (RFC 5869 §2.3).
fn expand(prk: &[u8], info: &[u8], len: usize) -> Result<Vec<u8>, TlsError> {
    if len > 255 * HASH_LEN {
        return Err(TlsError::Crypto("hkdf output too long"));
    }

    let key = hmac::Key::new(hmac::HMAC_SHA256, prk);
    let mut out = Vec::with_capacity(len);
    let mut previous: Vec<u8> = Vec::new();
    let mut counter: u8 = 1;

    while out.len() < len {
        let mut ctx = hmac::Context::with_key(&key);
        // T(n) = HMAC(prk, T(n-1) ‖ info ‖ n); T(0) is empty.
        ctx.update(&previous);
        ctx.update(info);
        ctx.update(&[counter]);

        previous = ctx.sign().as_ref().to_vec();
        out.extend_from_slice(&previous);
        counter += 1;
    }

    out.truncate(len);
    Ok(out)
}

/// HKDF-Expand-Label, the labelled wrapper TLS puts around HKDF-Expand.
///
/// The label is always prefixed with "tls13 " so keys derived for one
/// purpose can never collide with another's.
pub fn expand_label(
    secret: &[u8],
    label: &[u8],
    context: &[u8],
    len: usize,
) -> Result<Vec<u8>, TlsError> {
    let mut info = Vec::with_capacity(2 + 1 + 6 + label.len() + 1 + context.len());
    info.extend_from_slice(&(len as u16).to_be_bytes());
    info.push((6 + label.len()) as u8);
    info.extend_from_slice(b"tls13 ");
    info.extend_from_slice(label);
    info.push(context.len() as u8);
    info.extend_from_slice(context);

    expand(secret, &info, len)
}

/// `Derive-Secret(secret, label, transcript_hash)`.
fn derive_secret(secret: &[u8], label: &[u8], transcript: &[u8]) -> Result<Vec<u8>, TlsError> {
    expand_label(secret, label, transcript, HASH_LEN)
}

/// One direction's traffic keys.
pub struct TrafficKeys {
    pub key: Vec<u8>,
    pub iv: Vec<u8>,
    /// Kept so `Finished` can be computed and checked.
    pub secret: Vec<u8>,
}

/// Carries the schedule forward through its stages.
pub struct KeySchedule {
    suite: CipherSuite,
    /// The current stage's extracted secret.
    current: Vec<u8>,
}

impl KeySchedule {
    /// Starts the schedule: Extract with no PSK, so both salt and IKM are
    /// all zeroes.
    pub fn new(suite: CipherSuite) -> Self {
        let zeros = [0u8; HASH_LEN];
        Self {
            suite,
            current: extract(&zeros, &zeros),
        }
    }

    /// Mixes in the ECDHE shared secret, moving to the handshake stage.
    pub fn enter_handshake(&mut self, shared: &[u8]) -> Result<(), TlsError> {
        let salt = self.derive_for_next()?;
        self.current = extract(&salt, shared);
        Ok(())
    }

    /// Mixes in nothing, moving to the application stage.
    pub fn enter_application(&mut self) -> Result<(), TlsError> {
        let salt = self.derive_for_next()?;
        self.current = extract(&salt, &[0u8; HASH_LEN]);
        Ok(())
    }

    /// `Derive-Secret(., "derived", "")` — the bridge between stages.
    fn derive_for_next(&self) -> Result<Vec<u8>, TlsError> {
        let empty_hash = digest(&SHA256, b"");
        derive_secret(&self.current, b"derived", empty_hash.as_ref())
    }

    /// A traffic secret for this stage, bound to the transcript hash that
    /// fixes which messages it covers.
    pub fn traffic_secret(&self, label: &[u8], transcript: &[u8]) -> Result<Vec<u8>, TlsError> {
        derive_secret(&self.current, label, transcript)
    }

    /// Expands a traffic secret into the record-layer key and IV.
    pub fn keys_from_secret(&self, secret: &[u8]) -> Result<TrafficKeys, TlsError> {
        Ok(TrafficKeys {
            key: expand_label(secret, b"key", b"", self.suite.key_len())?,
            iv: expand_label(secret, b"iv", b"", 12)?,
            secret: secret.to_vec(),
        })
    }

    /// Both in one step, for the common case.
    pub fn traffic_keys(&self, label: &[u8], transcript: &[u8]) -> Result<TrafficKeys, TlsError> {
        let secret = self.traffic_secret(label, transcript)?;
        self.keys_from_secret(&secret)
    }
}

/// The `Finished` MAC: HMAC over the transcript hash, keyed by a value
/// derived from that side's traffic secret.
pub fn finished_mac(traffic_secret: &[u8], transcript: &[u8]) -> Result<Vec<u8>, TlsError> {
    let finished_key = expand_label(traffic_secret, b"finished", b"", HASH_LEN)?;
    let key = hmac::Key::new(hmac::HMAC_SHA256, &finished_key);
    Ok(hmac::sign(&key, transcript).as_ref().to_vec())
}
