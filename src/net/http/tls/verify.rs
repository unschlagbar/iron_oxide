//! Deciding whether to trust the peer.
//!
//! This is the security boundary of the whole module. Everything else can be
//! wrong in ways that produce a failed handshake; a mistake here produces a
//! *successful* handshake with the wrong party, which looks identical to a
//! correct one from the outside.
//!
//! So the options are deliberately narrow. [`Verifier::Roots`] validates a
//! chain up to roots the caller names, but it is not a system root store:
//! it rejects name constraints and unknown critical extensions instead of
//! implementing them, and does no revocation checks.

use super::cert::{Certificate, KeyAlgorithm, SignatureAlgorithm};
use super::TlsError;

/// Most intermediates a path may hold; public chains use one or two.
const MAX_INTERMEDIATES: usize = 6;

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

    /// Trust any chain that leads from the leaf to one of these roots.
    ///
    /// Survives certificate renewal, which a pin does not. Build it with
    /// [`Verifier::roots`] so a bad root fails at startup, not at connect.
    Roots(Vec<Certificate>),

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
    /// Parses DER root certificates into a [`Verifier::Roots`].
    pub fn roots(ders: &[Vec<u8>]) -> Result<Self, TlsError> {
        let roots = ders
            .iter()
            .map(|der| Certificate::parse(der))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self::Roots(roots))
    }

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

            Self::Roots(roots) => {
                if roots.is_empty() {
                    return Err(TlsError::BadCertificate(
                        "no trust anchors configured: set TlsConfig::verifier",
                    ));
                }
                check_leaf(leaf)?;
                let pool = &chain[1..chain.len().min(1 + 8)];
                if !find_path(leaf, 0, pool, 0, roots, now) {
                    return Err(TlsError::BadCertificate(
                        "server certificate does not chain to a trusted root",
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

fn check_leaf(leaf: &Certificate) -> Result<(), TlsError> {
    let ext = &leaf.extensions;
    if ext.is_ca {
        return Err(TlsError::BadCertificate("server certificate is a CA"));
    }
    if ext.server_auth == Some(false) {
        return Err(TlsError::BadCertificate("server certificate is not for server auth"));
    }
    if ext.unknown_critical {
        return Err(TlsError::BadCertificate("server certificate has an unknown critical extension"));
    }
    Ok(())
}

/// Searches for an issuer of `cert`, first among the roots, then through
/// unused certificates of `pool`; `below` counts intermediates under `cert`'s issuer.
fn find_path(
    cert: &Certificate,
    below: u32,
    pool: &[Certificate],
    used: u32,
    roots: &[Certificate],
    now: i64,
) -> bool {
    // A root is trusted by being configured, so only its key and name matter.
    if roots
        .iter()
        .any(|root| root.subject == cert.issuer && is_signed_by(cert, root))
    {
        return true;
    }
    if below as usize >= MAX_INTERMEDIATES {
        return false;
    }

    pool.iter().enumerate().any(|(i, ca)| {
        used & (1 << i) == 0
            && ca.subject == cert.issuer
            && can_issue(ca, below, now)
            && is_signed_by(cert, ca)
            && find_path(ca, below + 1, pool, used | (1 << i), roots, now)
    })
}

fn can_issue(ca: &Certificate, below: u32, now: i64) -> bool {
    let ext = &ca.extensions;
    ext.is_ca
        && ext.path_len.is_none_or(|max| below <= max)
        && ext.key_usage.is_none_or(|bits| bits & super::cert::Extensions::KEY_CERT_SIGN != 0)
        && ext.server_auth != Some(false)
        && !ext.name_constraints
        && !ext.unknown_critical
        && ca.is_valid_at(now)
}

fn is_signed_by(cert: &Certificate, issuer: &Certificate) -> bool {
    use ring::signature as sig;
    use KeyAlgorithm as K;
    use SignatureAlgorithm as S;

    let Some(alg) = cert.signature_algorithm else {
        return false;
    };
    let algorithm: &dyn sig::VerificationAlgorithm = match (alg, issuer.key_algorithm) {
        (S::RsaPkcs1Sha256, K::Rsa) => &sig::RSA_PKCS1_2048_8192_SHA256,
        (S::RsaPkcs1Sha384, K::Rsa) => &sig::RSA_PKCS1_2048_8192_SHA384,
        (S::RsaPkcs1Sha512, K::Rsa) => &sig::RSA_PKCS1_2048_8192_SHA512,
        (S::EcdsaSha256, K::EcdsaP256) => &sig::ECDSA_P256_SHA256_ASN1,
        (S::EcdsaSha256, K::EcdsaP384) => &sig::ECDSA_P384_SHA256_ASN1,
        (S::EcdsaSha384, K::EcdsaP256) => &sig::ECDSA_P256_SHA384_ASN1,
        (S::EcdsaSha384, K::EcdsaP384) => &sig::ECDSA_P384_SHA384_ASN1,
        (S::Ed25519, K::Ed25519) => &sig::ED25519,
        _ => return false,
    };
    sig::UnparsedPublicKey::new(algorithm, &issuer.public_key)
        .verify(&cert.tbs, &cert.signature)
        .is_ok()
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
