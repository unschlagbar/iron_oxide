//! Wire-format reading shared by the server and the client.
//!
//! Requests and responses differ only in their first line; headers and body
//! framing are identical, so both sides read them through these helpers.
//! Everything is generic over `BufRead` so the client can also run on top of
//! a buffered TLS stream later.

use std::io::{BufRead, Read};

use super::{Headers, HttpError, HttpResult, Limits};

/// Reads a line up to CRLF and returns it without the line ending.
///
/// A bare LF without CR is accepted too, because some peers send it that
/// way; the CR simply is not found then.
pub fn read_line(reader: &mut impl BufRead, max: usize) -> HttpResult<String> {
    let mut buf = Vec::with_capacity(128);

    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            // EOF. A partial line is broken; an empty one is a clean
            // connection shutdown.
            if buf.is_empty() {
                return Err(HttpError::ConnectionClosed);
            }
            return Err(HttpError::Malformed("unexpected EOF in line"));
        }

        match available.iter().position(|&b| b == b'\n') {
            Some(i) => {
                buf.extend_from_slice(&available[..i]);
                reader.consume(i + 1);
                break;
            }
            None => {
                let n = available.len();
                buf.extend_from_slice(available);
                reader.consume(n);
                if buf.len() > max {
                    return Err(HttpError::TooLarge("line"));
                }
            }
        }
    }

    if buf.len() > max {
        return Err(HttpError::TooLarge("line"));
    }
    if buf.last() == Some(&b'\r') {
        buf.pop();
    }

    String::from_utf8(buf).map_err(|_| HttpError::Malformed("line is not valid UTF-8"))
}

/// Reads a header block up to the terminating empty line.
pub fn read_headers(reader: &mut impl BufRead, limits: &Limits) -> HttpResult<Headers> {
    let mut headers = Headers::with_capacity(16);
    let mut consumed = 0;

    loop {
        let line = read_line(reader, limits.max_header_bytes.saturating_sub(consumed))?;
        if line.is_empty() {
            // An empty line terminates the header block.
            return Ok(headers);
        }

        consumed += line.len() + 2;
        if consumed > limits.max_header_bytes {
            return Err(HttpError::TooLarge("header block"));
        }
        if headers.len() >= limits.max_headers {
            return Err(HttpError::TooLarge("header count"));
        }

        let (name, value) = line
            .split_once(':')
            .ok_or(HttpError::Malformed("header without colon"))?;

        // No whitespace is allowed between name and colon. Letting that
        // through is the classic request-smuggling vector.
        if name.is_empty() || name.ends_with(' ') || name.ends_with('\t') {
            return Err(HttpError::Malformed("whitespace before colon"));
        }
        if !name.bytes().all(is_token_byte) {
            return Err(HttpError::Malformed("illegal byte in header name"));
        }

        headers.append(name, value.trim());
    }
}

/// Reads a body framed by `Transfer-Encoding` or `Content-Length`.
///
/// A message without either framing header has no body here. That is the
/// correct reading for requests; responses may instead be delimited by EOF,
/// which the client handles separately.
pub fn read_body(
    reader: &mut impl BufRead,
    headers: &Headers,
    limits: &Limits,
) -> HttpResult<Vec<u8>> {
    // Transfer-Encoding takes precedence over Content-Length. If both are
    // present this is a smuggling attempt and gets rejected.
    let chunked = headers.contains_token("Transfer-Encoding", "chunked");
    let content_length = headers.get("Content-Length");

    if chunked && content_length.is_some() {
        return Err(HttpError::Malformed(
            "both Transfer-Encoding and Content-Length present",
        ));
    }

    if chunked {
        return read_chunked_body(reader, limits);
    }

    let Some(raw_len) = content_length else {
        return Ok(Vec::new());
    };

    // Multiple conflicting Content-Length headers are equally suspicious.
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
    reader.read_exact(&mut body)?;
    Ok(body)
}

pub fn read_chunked_body(reader: &mut impl BufRead, limits: &Limits) -> HttpResult<Vec<u8>> {
    let mut body = Vec::new();

    loop {
        let size_line = read_line(reader, 1024)?;
        // The size may be followed by chunk extensions, which are ignored.
        let size_hex = size_line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_hex, 16)
            .map_err(|_| HttpError::Malformed("invalid chunk size"))?;

        if size == 0 {
            // Discard trailer headers up to the blank line.
            while !read_line(reader, limits.max_header_bytes)?.is_empty() {}
            return Ok(body);
        }

        if body.len() + size > limits.max_body {
            return Err(HttpError::TooLarge("chunked body"));
        }

        let start = body.len();
        body.resize(start + size, 0);
        reader.read_exact(&mut body[start..])?;

        // Every chunk ends with a CRLF that is not part of the content.
        let mut crlf = [0u8; 2];
        reader.read_exact(&mut crlf)?;
        if &crlf != b"\r\n" {
            return Err(HttpError::Malformed("chunk not terminated by CRLF"));
        }
    }
}

/// Reads until the peer closes the connection, capped by `max`.
///
/// This is the last-resort framing for HTTP/1.0-style responses that carry
/// neither Content-Length nor chunked encoding.
pub fn read_to_end_limited(reader: &mut impl BufRead, max: usize) -> HttpResult<Vec<u8>> {
    let mut body = Vec::new();
    // `take` caps the read itself, so an endless response cannot exhaust
    // memory before the limit is noticed.
    reader.take(max as u64 + 1).read_to_end(&mut body)?;
    if body.len() > max {
        return Err(HttpError::TooLarge("body"));
    }
    Ok(body)
}

pub fn is_token_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(
            b,
            b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'^' | b'_'
                | b'`' | b'|' | b'~'
        )
}
