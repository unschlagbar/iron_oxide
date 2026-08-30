use super::{HttpError, HttpResult};

/// A parsed request target: path plus raw query string.
#[derive(Debug, Clone)]
pub struct Uri {
    /// Percent-decoded, normalized path. Always starts with `/`.
    pub path: String,
    /// Raw query string without the `?`, not yet decoded.
    pub query: Option<String>,
}

impl Uri {
    pub fn parse(raw: &str) -> HttpResult<Self> {
        if raw.is_empty() {
            return Err(HttpError::Malformed("empty request target"));
        }

        // Absolute-form (`GET http://host/path HTTP/1.1`) is legal and used
        // by proxies. Scheme and authority are discarded here.
        let raw = if let Some(rest) = raw
            .strip_prefix("http://")
            .or_else(|| raw.strip_prefix("https://"))
        {
            match rest.find('/') {
                Some(i) => &rest[i..],
                None => "/",
            }
        } else {
            raw
        };

        let (path, query) = match raw.split_once('?') {
            Some((p, q)) => (p, Some(q.to_string())),
            None => (raw, None),
        };

        // Fragments do not belong on the wire, but some clients send them
        // anyway. Strip rather than reject.
        let path = path.split('#').next().unwrap_or(path);

        if !path.starts_with('/') {
            return Err(HttpError::Malformed("request target must start with /"));
        }

        Ok(Self {
            path: percent_decode(path)?,
            query,
        })
    }

    /// Iterates the query parameters as decoded key/value pairs.
    pub fn query_pairs(&self) -> impl Iterator<Item = (String, String)> + '_ {
        self.query
            .as_deref()
            .unwrap_or("")
            .split('&')
            .filter(|s| !s.is_empty())
            .map(|pair| {
                let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
                (decode_form(k), decode_form(v))
            })
    }

    pub fn query_param(&self, name: &str) -> Option<String> {
        self.query_pairs().find(|(k, _)| k == name).map(|(_, v)| v)
    }
}

/// Decodes `%XX` sequences. Invalid sequences are an error so that broken
/// input cannot silently slip through as a literal.
pub fn percent_decode(s: &str) -> HttpResult<String> {
    if !s.contains('%') {
        return Ok(s.to_string());
    }

    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(HttpError::Malformed("truncated percent-encoding"));
            }
            let hi = hex_val(bytes[i + 1]).ok_or(HttpError::Malformed("bad percent-encoding"))?;
            let lo = hex_val(bytes[i + 2]).ok_or(HttpError::Malformed("bad percent-encoding"))?;
            out.push(hi << 4 | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }

    String::from_utf8(out).map_err(|_| HttpError::Malformed("percent-encoding is not valid UTF-8"))
}

/// Like `percent_decode`, but also maps `+` to space and never fails.
/// This is the encoding used by forms and query strings.
fn decode_form(s: &str) -> String {
    let replaced = s.replace('+', " ");
    percent_decode(&replaced).unwrap_or(replaced)
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
