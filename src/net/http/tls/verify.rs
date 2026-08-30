//! Deciding whether to trust the peer.
//!
//! This is the security boundary of the whole module. Everything else can be
//! wrong in ways that produce a failed handshake; a mistake here produces a
//! *successful* handshake with the wrong party, which looks identical to a
//! correct one from the outside.
//!
//! So the options are deliberately narrow. There is no "verify against the
//! system root store" mode, because that needs full chain validation — path
//! building, basic constraints, key usage, name constraints, revocation —
//! and this crate does not implement it. Pretending otherwise would be the
//! dangerous kind of wrong.

use super::cert::Certificate;
use super::TlsError;

/// How a peer's certificate chain is judged.
#[derive(Debug, Clone)]
pub enum Verifier {
    /// Trust exactly these certificates, compared by their full DER bytes.
    ///
    /// The chain the server sends is irrelevant beyond its leaf: either the
    /// leaf is one you pinned or the handshake fails. This is the right
    /// model for talking to your own services, and it is what this client
    /// can actually enforce.
    ///
    /// An empty list rejects every peer, which is the default.
    Pinned(Vec<Vec<u8>>),

    /// Trust any certificate whose public key matches one of these
    /// (the subjectPublicKey bytes).
    ///
    /// Survives certificate renewal as long as the key is reused, which
    /// pinning the full DER does not.
    PinnedPublicKey(Vec<Vec<u8>>),

    /// Accept any certificate at all.
    ///
    /// This disables authentication completely: the connection is still
    /// encrypted, but you have no idea who is on the other end, and an
    /// attacker on the network path can read and rewrite everything by
    /// presenting their own certificate. Only reasonable against a server
    /// you also control on a network you trust, or when debugging.
    #[doc(hidden)]
    Insecure,
}

impl Verifier {
    /// Checks the chain the server presented.
    ///
    /// `chain` is in the order the peer sent it, so the leaf is first.
    /// `hostname` is the name the client asked for.
    pub fn verify(
        &self,
        chain: &[Certificate],
        hostname: &str,
        now: i64,
    ) -> Result<(), TlsError> {
        let leaf = chain
            .first()
            .ok_or(TlsError::BadCertificate("server sent an empty chain"))?;

        match self {
            Self::Insecure => return Ok(()),

            Self::Pinned(pins) => {
                if pins.is_empty() {
                    return Err(TlsError::BadCertificate(
                        "no trust anchors configured: set TlsConfig::verifier",
                    ));
                }
                if !pins.iter().any(|p| constant_time_eq(p, &leaf.der)) {
                    return Err(TlsError::BadCertificate(
                        "server certificate does not match any pin",
                    ));
                }
            }

            Self::PinnedPublicKey(keys) => {
                if keys.is_empty() {
                    return Err(TlsError::BadCertificate(
                        "no trust anchors configured: set TlsConfig::verifier",
                    ));
                }
                if !keys.iter().any(|k| constant_time_eq(k, &leaf.public_key)) {
                    return Err(TlsError::BadCertificate(
                        "server public key does not match any pin",
                    ));
                }
            }
        }

        // Expiry and hostname are checked even for a pinned certificate. A
        // pin says "this is the key I expect", not "ignore everything else",
        // and an expired or misdirected pin is still a signal something is
        // wrong.
        if !leaf.is_valid_at(now) {
            return Err(TlsError::BadCertificate(
                "server certificate is expired or not yet valid",
            ));
        }
        if !leaf.matches_hostname(hostname) {
            return Err(TlsError::BadCertificate(
                "server certificate is not valid for this hostname",
            ));
        }

        Ok(())
    }
}

/// Compares two byte strings without an early exit on the first difference.
///
/// The pins here are not secret, so this is defence in depth rather than a
/// strict requirement; it costs nothing and removes the question.
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

/// The current time as Unix seconds, for certificate validity.
pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        // A clock before 1970 means we cannot judge validity; 0 makes every
        // certificate look not-yet-valid, which fails closed.
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_pin_set_rejects() {
        let v = Verifier::Pinned(Vec::new());
        let err = v.verify(&[], "example.com", 0).unwrap_err();
        assert!(matches!(err, TlsError::BadCertificate(_)));
    }

    #[test]
    fn constant_time_eq_is_correct() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }
}
