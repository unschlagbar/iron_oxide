//! Just enough X.509 to pull the fields the handshake needs.
//!
//! This parses DER far enough to reach the public key, the validity window,
//! and the subject alternative names. It is **not** a chain validator: it
//! does not walk issuers, check basic constraints, key usage, name
//! constraints, or revocation. [`super::verify`] decides trust, and the
//! shipped verifiers only accept certificates they were told about by their
//! exact bytes, which is what makes that omission safe here.
//!
//! DER parsers are a well-known source of memory-safety and confusion bugs.
//! This one only ever takes subslices of the input, never copies by length
//! into a fixed buffer, and rejects anything it does not fully understand.

use super::TlsError;

/// A DER tag-length-value triple.
struct Tlv<'a> {
    tag: u8,
    value: &'a [u8],
}

/// Reads one TLV from the front of `input`, returning it and the rest.
fn read_tlv(input: &[u8]) -> Result<(Tlv<'_>, &[u8]), TlsError> {
    if input.len() < 2 {
        return Err(TlsError::Decode("der: truncated tag"));
    }

    let tag = input[0];
    let first = input[1];

    // Short form carries the length in the low seven bits; long form uses
    // them as a count of subsequent length bytes.
    let (len, rest) = if first & 0x80 == 0 {
        (first as usize, &input[2..])
    } else {
        let count = (first & 0x7f) as usize;
        // Anything above four bytes of length is far beyond any real
        // certificate and would risk overflowing `usize` on 32-bit.
        if count == 0 || count > 4 || input.len() < 2 + count {
            return Err(TlsError::Decode("der: bad length"));
        }
        let mut len = 0usize;
        for &b in &input[2..2 + count] {
            len = (len << 8) | b as usize;
        }
        (len, &input[2 + count..])
    };

    if rest.len() < len {
        return Err(TlsError::Decode("der: truncated value"));
    }

    Ok((
        Tlv {
            tag,
            value: &rest[..len],
        },
        &rest[len..],
    ))
}

/// Reads a TLV and requires a specific tag.
fn expect<'a>(input: &'a [u8], tag: u8, what: &'static str) -> Result<(Tlv<'a>, &'a [u8]), TlsError> {
    let (tlv, rest) = read_tlv(input)?;
    if tlv.tag != tag {
        let _ = what;
        return Err(TlsError::Decode("der: unexpected tag"));
    }
    Ok((tlv, rest))
}

const TAG_SEQUENCE: u8 = 0x30;
const TAG_SET: u8 = 0x31;
const TAG_INTEGER: u8 = 0x02;
const TAG_BIT_STRING: u8 = 0x03;
const TAG_OCTET_STRING: u8 = 0x04;
const TAG_OID: u8 = 0x06;
const TAG_UTC_TIME: u8 = 0x17;
const TAG_GENERALIZED_TIME: u8 = 0x18;

/// The signature algorithm a certificate's key can be used with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAlgorithm {
    Rsa,
    EcdsaP256,
    EcdsaP384,
    Ed25519,
}

/// A parsed end-entity certificate.
#[derive(Debug, Clone)]
pub struct Certificate {
    /// The original DER, kept because pinning compares these bytes and
    /// re-encoding is not guaranteed to round-trip.
    pub der: Vec<u8>,
    pub key_algorithm: KeyAlgorithm,
    /// The subjectPublicKey bit string contents.
    pub public_key: Vec<u8>,
    /// notBefore and notAfter as seconds since the Unix epoch.
    pub not_before: i64,
    pub not_after: i64,
    /// dNSName entries from the subjectAltName extension.
    pub dns_names: Vec<String>,
}

impl Certificate {
    pub fn parse(der: &[u8]) -> Result<Self, TlsError> {
        let (cert, trailing) = expect(der, TAG_SEQUENCE, "Certificate")?;
        if !trailing.is_empty() {
            return Err(TlsError::Decode("der: trailing data after certificate"));
        }

        // Certificate ::= SEQUENCE { tbsCertificate, signatureAlgorithm,
        //                            signatureValue }
        let (tbs, _rest) = expect(cert.value, TAG_SEQUENCE, "tbsCertificate")?;

        let mut body = tbs.value;

        // version [0] EXPLICIT, optional and defaulting to v1.
        if body.first() == Some(&0xa0) {
            let (_, rest) = read_tlv(body)?;
            body = rest;
        }

        let (_serial, body) = expect(body, TAG_INTEGER, "serialNumber")?;
        let (_sigalg, body) = expect(body, TAG_SEQUENCE, "signature")?;
        let (_issuer, body) = expect(body, TAG_SEQUENCE, "issuer")?;

        let (validity, body) = expect(body, TAG_SEQUENCE, "validity")?;
        let (not_before, not_after) = parse_validity(validity.value)?;

        let (_subject, body) = expect(body, TAG_SEQUENCE, "subject")?;

        let (spki, body) = expect(body, TAG_SEQUENCE, "subjectPublicKeyInfo")?;
        let (key_algorithm, public_key) = parse_spki(spki.value)?;

        // Remaining optional fields; only extensions [3] interest us.
        let dns_names = find_dns_names(body)?;

        Ok(Self {
            der: der.to_vec(),
            key_algorithm,
            public_key,
            not_before,
            not_after,
            dns_names,
        })
    }

    /// Whether `now` (Unix seconds) falls inside the validity window.
    pub fn is_valid_at(&self, now: i64) -> bool {
        now >= self.not_before && now <= self.not_after
    }

    /// Whether this certificate is valid for `hostname`.
    ///
    /// Only subjectAltName dNSName entries count. The Common Name fallback
    /// is deliberately not implemented: it has been deprecated for years and
    /// browsers no longer accept it.
    pub fn matches_hostname(&self, hostname: &str) -> bool {
        let hostname = hostname.trim_end_matches('.').to_ascii_lowercase();
        self.dns_names
            .iter()
            .any(|name| dns_name_matches(&name.to_ascii_lowercase(), &hostname))
    }
}

/// Matches a certificate DNS name against a hostname, allowing a single
/// leading wildcard label.
fn dns_name_matches(pattern: &str, hostname: &str) -> bool {
    let pattern = pattern.trim_end_matches('.');
    if pattern.is_empty() || hostname.is_empty() {
        return false;
    }

    let Some(suffix) = pattern.strip_prefix("*.") else {
        return pattern == hostname;
    };

    // A wildcard matches exactly one label, and never the leading dot, so
    // `*.example.com` covers `a.example.com` but not `example.com` or
    // `a.b.example.com`.
    let Some(rest) = hostname.split_once('.').map(|(_, rest)| rest) else {
        return false;
    };
    // Refuse a bare `*.com`-style pattern: the remainder must itself have a
    // dot, or one certificate would cover a whole TLD.
    if !suffix.contains('.') {
        return false;
    }
    rest == suffix
}

fn parse_validity(input: &[u8]) -> Result<(i64, i64), TlsError> {
    let (before, rest) = read_tlv(input)?;
    let (after, _) = read_tlv(rest)?;
    Ok((parse_time(&before)?, parse_time(&after)?))
}

/// Parses UTCTime or GeneralizedTime into Unix seconds.
fn parse_time(tlv: &Tlv<'_>) -> Result<i64, TlsError> {
    let s = std::str::from_utf8(tlv.value).map_err(|_| TlsError::Decode("der: bad time"))?;

    let (year, rest) = match tlv.tag {
        TAG_UTC_TIME => {
            // Two-digit year; the pivot at 50 is what RFC 5280 specifies.
            if s.len() < 13 {
                return Err(TlsError::Decode("der: short utctime"));
            }
            let yy: i64 = s[0..2].parse().map_err(|_| TlsError::Decode("der: bad year"))?;
            (if yy >= 50 { 1900 + yy } else { 2000 + yy }, &s[2..])
        }
        TAG_GENERALIZED_TIME => {
            if s.len() < 15 {
                return Err(TlsError::Decode("der: short generalizedtime"));
            }
            let y: i64 = s[0..4].parse().map_err(|_| TlsError::Decode("der: bad year"))?;
            (y, &s[4..])
        }
        _ => return Err(TlsError::Decode("der: not a time")),
    };

    let num = |r: &str, at: usize| -> Result<i64, TlsError> {
        r.get(at..at + 2)
            .and_then(|s| s.parse().ok())
            .ok_or(TlsError::Decode("der: bad time field"))
    };

    let month = num(rest, 0)?;
    let day = num(rest, 2)?;
    let hour = num(rest, 4)?;
    let minute = num(rest, 6)?;
    let second = num(rest, 8)?;

    Ok(days_from_civil(year, month, day) * 86400 + hour * 3600 + minute * 60 + second)
}

/// Days since the Unix epoch, by Howard Hinnant's civil-date algorithm.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Pulls the algorithm and public key out of a SubjectPublicKeyInfo.
fn parse_spki(input: &[u8]) -> Result<(KeyAlgorithm, Vec<u8>), TlsError> {
    let (alg, rest) = expect(input, TAG_SEQUENCE, "algorithm")?;
    let (key_bits, _) = expect(rest, TAG_BIT_STRING, "subjectPublicKey")?;

    let (oid, alg_rest) = expect(alg.value, TAG_OID, "algorithm oid")?;

    const OID_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
    const OID_EC: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01];
    const OID_ED25519: &[u8] = &[0x2b, 0x65, 0x70];
    const OID_P256: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07];
    const OID_P384: &[u8] = &[0x2b, 0x81, 0x04, 0x00, 0x22];

    let algorithm = match oid.value {
        OID_RSA => KeyAlgorithm::Rsa,
        OID_ED25519 => KeyAlgorithm::Ed25519,
        OID_EC => {
            // The curve is a parameter, and the two curves differ enough
            // that guessing is not an option.
            let (curve, _) = expect(alg_rest, TAG_OID, "curve oid")?;
            match curve.value {
                OID_P256 => KeyAlgorithm::EcdsaP256,
                OID_P384 => KeyAlgorithm::EcdsaP384,
                _ => return Err(TlsError::Unsupported("certificate uses an unknown curve")),
            }
        }
        _ => return Err(TlsError::Unsupported("certificate key algorithm")),
    };

    // A BIT STRING begins with a count of unused trailing bits, which is
    // always zero for keys and is not part of the key itself.
    let key = key_bits
        .value
        .split_first()
        .filter(|(unused, _)| **unused == 0)
        .map(|(_, rest)| rest.to_vec())
        .ok_or(TlsError::Decode("der: bad public key bit string"))?;

    Ok((algorithm, key))
}

/// Walks the optional trailing fields of a TBSCertificate looking for the
/// subjectAltName extension.
fn find_dns_names(mut input: &[u8]) -> Result<Vec<String>, TlsError> {
    const OID_SUBJECT_ALT_NAME: &[u8] = &[0x55, 0x1d, 0x11];

    while !input.is_empty() {
        let (tlv, rest) = read_tlv(input)?;
        input = rest;

        // extensions [3] EXPLICIT SEQUENCE OF Extension
        if tlv.tag != 0xa3 {
            continue;
        }

        let (list, _) = expect(tlv.value, TAG_SEQUENCE, "extensions")?;
        let mut items = list.value;

        while !items.is_empty() {
            let (ext, rest) = expect(items, TAG_SEQUENCE, "extension")?;
            items = rest;

            let (oid, after_oid) = expect(ext.value, TAG_OID, "extension oid")?;
            if oid.value != OID_SUBJECT_ALT_NAME {
                continue;
            }

            // An optional critical BOOLEAN may sit before the value.
            let after_oid = match read_tlv(after_oid) {
                Ok((t, rest)) if t.tag == 0x01 => rest,
                _ => after_oid,
            };

            let (value, _) = expect(after_oid, TAG_OCTET_STRING, "extension value")?;
            let (names, _) = expect(value.value, TAG_SEQUENCE, "GeneralNames")?;

            let mut out = Vec::new();
            let mut entries = names.value;
            while !entries.is_empty() {
                let (entry, rest) = read_tlv(entries)?;
                entries = rest;
                // dNSName is context tag 2, IA5String.
                if entry.tag == 0x82
                    && let Ok(name) = std::str::from_utf8(entry.value)
                {
                    out.push(name.to_string());
                }
            }
            return Ok(out);
        }
    }

    Ok(Vec::new())
}

/// Ignored, but kept so the SET tag constant documents the structures we
/// skip over inside issuer and subject names.
#[allow(dead_code)]
const _: u8 = TAG_SET;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards_match_one_label_only() {
        assert!(dns_name_matches("*.example.com", "a.example.com"));
        assert!(!dns_name_matches("*.example.com", "example.com"));
        assert!(!dns_name_matches("*.example.com", "a.b.example.com"));
        assert!(dns_name_matches("example.com", "example.com"));
    }

    #[test]
    fn wildcard_cannot_cover_a_tld() {
        assert!(!dns_name_matches("*.com", "example.com"));
    }

    #[test]
    fn civil_dates_match_known_epochs() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11017);
        assert_eq!(days_from_civil(2024, 1, 1), 19723);
    }
}
