use std::io::{self, Write};

use super::{Headers, Status};

#[derive(Debug)]
pub struct Response {
    pub status: Status,
    pub headers: Headers,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: Status) -> Self {
        Self {
            status,
            headers: Headers::with_capacity(8),
            body: Vec::new(),
        }
    }

    pub fn ok() -> Self {
        Self::new(Status::Ok)
    }

    /// A response with a body and an explicit Content-Type.
    pub fn bytes(status: Status, content_type: &str, body: impl Into<Vec<u8>>) -> Self {
        let mut res = Self::new(status);
        res.headers.set("Content-Type", content_type);
        res.body = body.into();
        res
    }

    pub fn text(body: impl Into<Vec<u8>>) -> Self {
        Self::bytes(Status::Ok, "text/plain; charset=utf-8", body)
    }

    pub fn html(body: impl Into<Vec<u8>>) -> Self {
        Self::bytes(Status::Ok, "text/html; charset=utf-8", body)
    }

    pub fn json(body: impl Into<Vec<u8>>) -> Self {
        Self::bytes(Status::Ok, "application/json", body)
    }

    /// A short error page carrying the reason phrase as text.
    pub fn error(status: Status) -> Self {
        let body = format!("{} {}\n", status.code(), status.reason());
        Self::bytes(status, "text/plain; charset=utf-8", body)
    }

    pub fn redirect(status: Status, location: &str) -> Self {
        let mut res = Self::new(status);
        res.headers.set("Location", location);
        res
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.set(name, value);
        self
    }

    /// Writes the complete response to the stream.
    ///
    /// `include_body` is false for HEAD: headers are still sent including
    /// Content-Length, but the body is omitted.
    pub fn write_to(
        &self,
        out: &mut impl Write,
        include_body: bool,
        keep_alive: bool,
    ) -> io::Result<()> {
        let send_body = include_body && self.status.allows_body();

        let mut head = Vec::with_capacity(256 + self.headers.len() * 32);
        head.extend_from_slice(b"HTTP/1.1 ");
        head.extend_from_slice(self.status.code().to_string().as_bytes());
        head.push(b' ');
        head.extend_from_slice(self.status.reason().as_bytes());
        head.extend_from_slice(b"\r\n");

        // A 101 hands the connection to another protocol. Its Connection and
        // Upgrade headers are part of the handshake and must pass through
        // untouched, so the server does not get to rewrite them.
        let is_upgrade = self.status == Status::SwitchingProtocols;

        for (name, value) in self.headers.iter() {
            // Content-Length and Connection are set by the server itself so
            // they cannot appear twice or contradict each other.
            if !is_upgrade
                && (name.eq_ignore_ascii_case("Content-Length")
                    || name.eq_ignore_ascii_case("Connection"))
            {
                continue;
            }
            head.extend_from_slice(name.as_bytes());
            head.extend_from_slice(b": ");
            head.extend_from_slice(value.as_bytes());
            head.extend_from_slice(b"\r\n");
        }

        if self.status.allows_body() {
            head.extend_from_slice(b"Content-Length: ");
            head.extend_from_slice(self.body.len().to_string().as_bytes());
            head.extend_from_slice(b"\r\n");
        }

        if !is_upgrade {
            head.extend_from_slice(if keep_alive {
                b"Connection: keep-alive\r\n"
            } else {
                b"Connection: close\r\n"
            });
        }
        head.extend_from_slice(b"\r\n");

        // Headers and small bodies go out in a single write so that a response
        // does not cost two TCP packets.
        if send_body && self.body.len() <= 64 * 1024 {
            head.extend_from_slice(&self.body);
            out.write_all(&head)?;
        } else {
            out.write_all(&head)?;
            if send_body {
                out.write_all(&self.body)?;
            }
        }

        out.flush()
    }
}
