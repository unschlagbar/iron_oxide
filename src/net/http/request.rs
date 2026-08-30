use std::io::BufRead;

use super::io::{read_body, read_line};
use super::{Headers, HttpError, HttpResult, Method, Uri};

/// Limits beyond which a request is rejected. Without them a client could
/// exhaust server memory with an endless stream of headers.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_request_line: usize,
    pub max_header_bytes: usize,
    pub max_headers: usize,
    pub max_body: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_request_line: 8 * 1024,
            max_header_bytes: 64 * 1024,
            max_headers: 128,
            max_body: 16 * 1024 * 1024,
        }
    }
}

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub uri: Uri,
    /// Minor version of HTTP/1.x. 0 = HTTP/1.0, 1 = HTTP/1.1.
    pub version_minor: u8,
    pub headers: Headers,
    pub body: Vec<u8>,
}

impl Request {
    /// Reads exactly one request from the stream.
    ///
    /// The reader is reused across calls because with keep-alive it may
    /// already hold buffered bytes of the next request. It is generic so the
    /// same parser serves plain and TLS connections.
    pub fn read_from(reader: &mut impl BufRead, limits: &Limits) -> HttpResult<Self> {
        let request_line = read_line(reader, limits.max_request_line)?;
        if request_line.is_empty() {
            return Err(HttpError::ConnectionClosed);
        }

        let (method, uri, version_minor) = parse_request_line(&request_line)?;
        let headers = super::io::read_headers(reader, limits)?;
        let body = read_body(reader, &headers, limits)?;

        Ok(Self {
            method,
            uri,
            version_minor,
            headers,
            body,
        })
    }

    pub fn path(&self) -> &str {
        &self.uri.path
    }

    /// Whether the connection should stay open after this request.
    /// HTTP/1.1 defaults to keep-alive, HTTP/1.0 does not.
    pub fn wants_keep_alive(&self) -> bool {
        if self.headers.contains_token("Connection", "close") {
            return false;
        }
        if self.version_minor == 0 {
            return self.headers.contains_token("Connection", "keep-alive");
        }
        true
    }

    /// The handshake key if this request is a WebSocket upgrade.
    pub fn websocket_key(&self) -> Option<&str> {
        if self.headers.contains_token("Connection", "upgrade")
            && self
                .headers
                .get("Upgrade")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("websocket"))
        {
            self.headers.get("Sec-WebSocket-Key").map(str::trim)
        } else {
            None
        }
    }

    pub fn body_as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.body).ok()
    }
}

fn parse_request_line(line: &str) -> HttpResult<(Method, Uri, u8)> {
    let mut parts = line.split(' ');

    let method = parts.next().ok_or(HttpError::Malformed("missing method"))?;
    let target = parts
        .next()
        .ok_or(HttpError::Malformed("missing request target"))?;
    let version = parts
        .next()
        .ok_or(HttpError::Malformed("missing http version"))?;

    if parts.next().is_some() {
        return Err(HttpError::Malformed("too many parts in request line"));
    }

    let version_minor = match version {
        "HTTP/1.1" => 1,
        "HTTP/1.0" => 0,
        _ => return Err(HttpError::UnsupportedVersion),
    };

    Ok((
        Method::parse(method.as_bytes())?,
        Uri::parse(target)?,
        version_minor,
    ))
}
