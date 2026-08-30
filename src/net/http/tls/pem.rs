//! PEM decoding, so certificates and keys can be loaded from the files
//! `openssl` and every ACME client actually produce.
//!
//! PEM is a base64 body between `-----BEGIN X-----` and `-----END X-----`
//! lines. Anything outside those markers is ignored, which is what allows
//! the comments and metadata real certificate files carry.

use super::TlsError;

/// Extracts every block with the given label, in file order.
pub fn extract(input: &[u8], label: &str) -> Result<Vec<Vec<u8>>, TlsError> {
    let text = std::str::from_utf8(input).map_err(|_| TlsError::Decode("pem is not utf-8"))?;

    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");

    let mut out = Vec::new();
    let mut rest = text;

    while let Some(start) = rest.find(&begin) {
        let after = &rest[start + begin.len()..];
        let Some(stop) = after.find(&end) else {
            return Err(TlsError::Decode("pem block is not terminated"));
        };

        let body: String = after[..stop].chars().filter(|c| !c.is_whitespace()).collect();
        out.push(base64_decode(&body)?);

        rest = &after[stop + end.len()..];
    }

    Ok(out)
}

/// Reads a certificate chain from PEM, leaf first.
pub fn certificates(input: &[u8]) -> Result<Vec<Vec<u8>>, TlsError> {
    let certs = extract(input, "CERTIFICATE")?;
    if certs.is_empty() {
        return Err(TlsError::Decode("no CERTIFICATE block found"));
    }
    Ok(certs)
}

/// Reads the first private key from PEM.
///
/// Accepts PKCS#8 (`PRIVATE KEY`), the form every modern tool emits. The
/// legacy `RSA PRIVATE KEY` and `EC PRIVATE KEY` forms are recognised well
/// enough to produce a useful error rather than a confusing parse failure.
pub fn private_key(input: &[u8]) -> Result<Vec<u8>, TlsError> {
    if let Some(key) = extract(input, "PRIVATE KEY")?.into_iter().next() {
        return Ok(key);
    }

    if !extract(input, "RSA PRIVATE KEY")?.is_empty()
        || !extract(input, "EC PRIVATE KEY")?.is_empty()
    {
        return Err(TlsError::Unsupported(
            "key is in the legacy PEM format; convert it with \
             `openssl pkcs8 -topk8 -nocrypt -in key.pem -out key-pkcs8.pem`",
        ));
    }

    Err(TlsError::Decode("no PRIVATE KEY block found"))
}

/// Decodes standard base64, rejecting anything malformed.
fn base64_decode(s: &str) -> Result<Vec<u8>, TlsError> {
    fn value(b: u8) -> Option<u8> {
        Some(match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        })
    }

    let bytes = s.as_bytes();
    let body = bytes.strip_suffix(b"==").or_else(|| bytes.strip_suffix(b"="));
    let (body, padding) = match body {
        Some(b) if bytes.ends_with(b"==") => (b, 2),
        Some(b) => (b, 1),
        None => (bytes, 0),
    };

    let mut out = Vec::with_capacity(body.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0;

    for &b in body {
        let v = value(b).ok_or(TlsError::Decode("pem: invalid base64"))? as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }

    // Padding must line up with what was actually decoded.
    if padding == 1 && body.len() % 4 != 3 {
        return Err(TlsError::Decode("pem: bad base64 padding"));
    }
    if padding == 2 && body.len() % 4 != 2 {
        return Err(TlsError::Decode("pem: bad base64 padding"));
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_base64_with_padding() {
        assert_eq!(base64_decode("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(base64_decode("aGVsbG8h").unwrap(), b"hello!");
        assert_eq!(base64_decode("aGk=").unwrap(), b"hi");
        assert_eq!(base64_decode("").unwrap(), b"");
    }

    #[test]
    fn rejects_invalid_base64() {
        assert!(base64_decode("aGVsbG8*").is_err());
    }

    #[test]
    fn extracts_multiple_blocks() {
        let pem = "\
noise before
-----BEGIN CERTIFICATE-----
aGVsbG8=
-----END CERTIFICATE-----
between
-----BEGIN CERTIFICATE-----
aGk=
-----END CERTIFICATE-----
";
        let blocks = extract(pem.as_bytes(), "CERTIFICATE").unwrap();
        assert_eq!(blocks, vec![b"hello".to_vec(), b"hi".to_vec()]);
    }

    #[test]
    fn legacy_key_format_gives_a_useful_error() {
        let pem = "-----BEGIN RSA PRIVATE KEY-----\naGk=\n-----END RSA PRIVATE KEY-----";
        let err = private_key(pem.as_bytes()).unwrap_err();
        assert!(matches!(err, TlsError::Unsupported(_)), "got {err:?}");
    }
}
