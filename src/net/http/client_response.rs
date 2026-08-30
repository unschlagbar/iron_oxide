//! The response side of the client.

use std::io::BufRead;

use super::io::{read_chunked_body, read_headers, read_line, read_to_end_limited};
use super::{Headers, HttpError, HttpResult, Limits, Method, Status};

/// A response as received by the client.
///
/// The status is kept as a raw code because a server may legitimately send
/// any code, including ones [`Status`] does not name. Use [`Self::status`]
/// when a typed value is needed.
#[derive(Debug)]
pub struct ClientResponse {
    pub code: u16,
    pub reason: String,
    /// Minor version of HTTP/1.x. 0 = HTTP/1.0, 1 = HTTP/1.1.
    pub version_minor: u8,
    pub headers: Headers,
    pub body: Vec<u8>,
}

impl ClientResponse {
    /// Reads one complete response.
    ///
    /// `method` decides body framing: the reply to a HEAD never has a body,
    /// no matter what Content-Length claims.
    pub fn read_from(
        reader: &mut impl BufRead,
        method: Method,
        limits: &Limits,
    ) -> HttpResult<Self> {
        let (code, reason, version_minor) = read_status_line(reader, limits)?;
        let headers = read_headers(reader, limits)?;
        let body = read_response_body(reader, code, method, &headers, limits)?;

        Ok(Self {
            code,
            reason,
            version_minor,
            headers,
            body,
        })
    }

    /// The typed status, if it is one of the codes this crate names.
    pub fn status(&self) -> Option<Status> {
        Some(match self.code {
            101 => Status::SwitchingProtocols,
            200 => Status::Ok,
            201 => Status::Created,
            204 => Status::NoContent,
            206 => Status::PartialContent,
            301 => Status::MovedPermanently,
            302 => Status::Found,
            304 => Status::NotModified,
            400 => Status::BadRequest,
            401 => Status::Unauthorized,
            403 => Status::Forbidden,
            404 => Status::NotFound,
            405 => Status::MethodNotAllowed,
            413 => Status::PayloadTooLarge,
            414 => Status::UriTooLong,
            416 => Status::RangeNotSatisfiable,
            500 => Status::InternalServerError,
            501 => Status::NotImplemented,
            503 => Status::ServiceUnavailable,
            505 => Status::HttpVersionNotSupported,
            _ => return None,
        })
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.code)
    }

    pub fn is_redirect(&self) -> bool {
        matches!(self.code, 301 | 302 | 303 | 307 | 308)
    }

    /// Fails on a non-2xx status so callers can use `?` instead of matching.
    pub fn error_for_status(self) -> HttpResult<Self> {
        if self.is_success() {
            Ok(self)
        } else {
            Err(HttpError::Status {
                code: self.code,
                reason: self.reason,
            })
        }
    }

    pub fn body_as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.body).ok()
    }

    /// The body as text, with invalid UTF-8 replaced rather than rejected.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// Whether the connection may be reused after this response.
    pub fn keeps_alive(&self) -> bool {
        if self.headers.contains_token("Connection", "close") {
            return false;
        }
        if self.version_minor == 0 {
            return self.headers.contains_token("Connection", "keep-alive");
        }
        true
    }
}

fn read_status_line(reader: &mut impl BufRead, limits: &Limits) -> HttpResult<(u16, String, u8)> {
    let line = read_line(reader, limits.max_request_line)?;
    if line.is_empty() {
        return Err(HttpError::ConnectionClosed);
    }

    let mut parts = line.splitn(3, ' ');

    let version = parts
        .next()
        .ok_or(HttpError::Malformed("missing http version"))?;
    let code = parts
        .next()
        .ok_or(HttpError::Malformed("missing status code"))?;

    let version_minor = match version {
        "HTTP/1.1" => 1,
        "HTTP/1.0" => 0,
        _ => return Err(HttpError::UnsupportedVersion),
    };

    let code: u16 = code
        .parse()
        .map_err(|_| HttpError::Malformed("invalid status code"))?;
    if !(100..1000).contains(&code) {
        return Err(HttpError::Malformed("status code out of range"));
    }

    // The reason phrase is optional and purely informational.
    Ok((code, parts.next().unwrap_or("").trim().to_string(), version_minor))
}

/// Applies RFC 9112 message framing for responses.
fn read_response_body(
    reader: &mut impl BufRead,
    code: u16,
    method: Method,
    headers: &Headers,
    limits: &Limits,
) -> HttpResult<Vec<u8>> {
    // These never carry a body, and any Content-Length on them is to be
    // ignored rather than trusted.
    let bodyless = matches!(code, 100..=199 | 204 | 304) || method == Method::Head;
    if bodyless {
        return Ok(Vec::new());
    }

    if headers.contains_token("Transfer-Encoding", "chunked") {
        return read_chunked_body(reader, limits);
    }

    if let Some(raw_len) = headers.get("Content-Length") {
        if headers.get_all("Content-Length").count() > 1 {
            return Err(HttpError::Malformed("duplicate Content-Length"));
        }

        let len: usize = raw_len
            .trim()
            .parse()
            .map_err(|_| HttpError::Malformed("invalid Content-Length"))?;
        if len > limits.max_body {
            return Err(HttpError::TooLarge("body"));
        }

        let mut body = vec![0u8; len];
        std::io::Read::read_exact(reader, &mut body)?;
        return Ok(body);
    }

    // No framing header at all: the body runs until the peer closes.
    read_to_end_limited(reader, limits.max_body)
}
