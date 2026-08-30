//! A blocking HTTP/1.1 client.
//!
//! No TLS: `https://` URLs are rejected at connect time rather than silently
//! downgraded, since a silent downgrade is worse than a clear error. Adding
//! TLS means wrapping the stream in [`Connection`] and nothing else, because
//! all parsing above it is generic over `BufRead`.

use std::io::{BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use super::transport::Transport;
use super::url::Scheme;
use super::{ClientResponse, Headers, HttpError, HttpResult, Limits, Method, Url};

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub limits: Limits,
    pub connect_timeout: Option<Duration>,
    pub read_timeout: Option<Duration>,
    pub write_timeout: Option<Duration>,
    /// How many redirects to follow before giving up. 0 disables following.
    pub max_redirects: usize,
    /// Sent as `User-Agent` unless the caller sets their own.
    pub user_agent: String,
    /// How TLS decides whether to trust the peer.
    #[cfg(feature = "tls")]
    pub tls: super::tls::TlsConfig,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            limits: Limits::default(),
            connect_timeout: Some(Duration::from_secs(10)),
            read_timeout: Some(Duration::from_secs(30)),
            write_timeout: Some(Duration::from_secs(30)),
            max_redirects: 10,
            user_agent: concat!("iron_oxide/", env!("CARGO_PKG_VERSION")).to_string(),
            #[cfg(feature = "tls")]
            tls: super::tls::TlsConfig::default(),
        }
    }
}

/// An outgoing request.
///
/// Built with the `Client` shorthands or [`ClientRequest::new`], then sent
/// with [`Client::send`].
#[derive(Debug, Clone)]
pub struct ClientRequest {
    pub method: Method,
    pub url: Url,
    pub headers: Headers,
    pub body: Vec<u8>,
}

impl ClientRequest {
    pub fn new(method: Method, url: Url) -> Self {
        Self {
            method,
            url,
            headers: Headers::with_capacity(8),
            body: Vec::new(),
        }
    }

    pub fn parse(method: Method, url: &str) -> HttpResult<Self> {
        Ok(Self::new(method, Url::parse(url)?))
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.set(name, value);
        self
    }

    /// Sets the body and its Content-Type.
    pub fn body(mut self, content_type: &str, body: impl Into<Vec<u8>>) -> Self {
        self.headers.set("Content-Type", content_type);
        self.body = body.into();
        self
    }

    pub fn json(self, body: impl Into<Vec<u8>>) -> Self {
        self.body("application/json", body)
    }

    pub fn text(self, body: impl Into<Vec<u8>>) -> Self {
        self.body("text/plain; charset=utf-8", body)
    }

    /// Sets a form-encoded body from key/value pairs.
    pub fn form<K: AsRef<str>, V: AsRef<str>>(self, pairs: impl IntoIterator<Item = (K, V)>) -> Self {
        let mut encoded = String::new();
        for (k, v) in pairs {
            if !encoded.is_empty() {
                encoded.push('&');
            }
            encoded.push_str(&form_encode(k.as_ref()));
            encoded.push('=');
            encoded.push_str(&form_encode(v.as_ref()));
        }
        self.body("application/x-www-form-urlencoded", encoded)
    }

    /// Serializes the request head and body onto the wire.
    fn write_to(&self, out: &mut impl Write, config: &ClientConfig) -> std::io::Result<()> {
        let mut head = Vec::with_capacity(256 + self.headers.len() * 32);

        head.extend_from_slice(self.method.as_str().as_bytes());
        head.push(b' ');
        head.extend_from_slice(self.url.request_target().as_bytes());
        head.extend_from_slice(b" HTTP/1.1\r\n");

        // Host is mandatory in HTTP/1.1 and derived from the URL, so it can
        // never contradict where the connection actually went.
        head.extend_from_slice(b"Host: ");
        head.extend_from_slice(self.url.host_header().as_bytes());
        head.extend_from_slice(b"\r\n");

        for (name, value) in self.headers.iter() {
            // Framing and routing headers belong to the client alone.
            if name.eq_ignore_ascii_case("Host")
                || name.eq_ignore_ascii_case("Content-Length")
                || name.eq_ignore_ascii_case("Connection")
                || name.eq_ignore_ascii_case("Transfer-Encoding")
            {
                continue;
            }
            head.extend_from_slice(name.as_bytes());
            head.extend_from_slice(b": ");
            head.extend_from_slice(value.as_bytes());
            head.extend_from_slice(b"\r\n");
        }

        if !self.headers.contains("User-Agent") {
            head.extend_from_slice(b"User-Agent: ");
            head.extend_from_slice(config.user_agent.as_bytes());
            head.extend_from_slice(b"\r\n");
        }

        // A Content-Length is sent whenever a body exists. Methods that
        // normally have none stay clean because their body is empty.
        if !self.body.is_empty() {
            head.extend_from_slice(b"Content-Length: ");
            head.extend_from_slice(self.body.len().to_string().as_bytes());
            head.extend_from_slice(b"\r\n");
        }

        // Connection reuse is not implemented, so say so instead of letting
        // the peer hold the socket open pointlessly.
        head.extend_from_slice(b"Connection: close\r\n\r\n");

        if !self.body.is_empty() && self.body.len() <= 64 * 1024 {
            head.extend_from_slice(&self.body);
            out.write_all(&head)?;
        } else {
            out.write_all(&head)?;
            if !self.body.is_empty() {
                out.write_all(&self.body)?;
            }
        }

        out.flush()
    }
}

/// A single connection to one host, for callers that want to drive the
/// exchange themselves.
///
/// The transport is owned rather than split into halves, because a TLS
/// session has a single cipher state that cannot be cloned.
pub struct Connection {
    reader: BufReader<Transport>,
}

impl Connection {
    pub fn connect(url: &Url, config: &ClientConfig) -> HttpResult<Self> {
        let stream = connect_tcp(url, config)?;
        stream.set_read_timeout(config.read_timeout)?;
        stream.set_write_timeout(config.write_timeout)?;
        let _ = stream.set_nodelay(true);

        let transport = match url.scheme {
            Scheme::Http => Transport::Plain(stream),
            Scheme::Https => connect_tls(stream, url, config)?,
        };

        Ok(Self {
            reader: BufReader::with_capacity(8 * 1024, transport),
        })
    }

    /// Sends one request and reads its response.
    pub fn request(
        &mut self,
        req: &ClientRequest,
        config: &ClientConfig,
    ) -> HttpResult<ClientResponse> {
        req.write_to(self.reader.get_mut(), config)?;
        ClientResponse::read_from(&mut self.reader, req.method, &config.limits)
    }

    pub fn is_tls(&self) -> bool {
        self.reader.get_ref().is_tls()
    }
}

#[cfg(feature = "tls")]
fn connect_tls(stream: TcpStream, url: &Url, config: &ClientConfig) -> HttpResult<Transport> {
    let tls = super::tls::TlsStream::connect(stream, &url.host, &config.tls)?;
    Ok(Transport::Tls(Box::new(tls)))
}

#[cfg(not(feature = "tls"))]
fn connect_tls(_stream: TcpStream, _url: &Url, _config: &ClientConfig) -> HttpResult<Transport> {
    Err(HttpError::Io(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "https requires the `tls` feature",
    )))
}

fn connect_tcp(url: &Url, config: &ClientConfig) -> std::io::Result<TcpStream> {
    let Some(timeout) = config.connect_timeout else {
        return TcpStream::connect(url.socket_addr());
    };

    // `connect_timeout` takes a single resolved address, so DNS results are
    // tried in turn and the last error is reported if all of them fail.
    let addrs = url.socket_addr().to_socket_addrs()?;
    let mut last_err = None;

    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => return Ok(stream),
            Err(e) => last_err = Some(e),
        }
    }

    Err(last_err.unwrap_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "host resolved to no addresses")
    }))
}

/// The entry point for sending requests.
#[derive(Debug, Clone, Default)]
pub struct Client {
    config: ClientConfig,
}

impl Client {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: ClientConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    pub fn get(&self, url: &str) -> HttpResult<ClientResponse> {
        self.send(ClientRequest::parse(Method::Get, url)?)
    }

    pub fn head(&self, url: &str) -> HttpResult<ClientResponse> {
        self.send(ClientRequest::parse(Method::Head, url)?)
    }

    pub fn delete(&self, url: &str) -> HttpResult<ClientResponse> {
        self.send(ClientRequest::parse(Method::Delete, url)?)
    }

    pub fn post(&self, url: &str, content_type: &str, body: impl Into<Vec<u8>>) -> HttpResult<ClientResponse> {
        self.send(ClientRequest::parse(Method::Post, url)?.body(content_type, body))
    }

    pub fn put(&self, url: &str, content_type: &str, body: impl Into<Vec<u8>>) -> HttpResult<ClientResponse> {
        self.send(ClientRequest::parse(Method::Put, url)?.body(content_type, body))
    }

    pub fn patch(&self, url: &str, content_type: &str, body: impl Into<Vec<u8>>) -> HttpResult<ClientResponse> {
        self.send(ClientRequest::parse(Method::Patch, url)?.body(content_type, body))
    }

    /// Sends a request, following redirects up to the configured limit.
    ///
    /// Each hop opens a fresh connection; there is no connection pool.
    pub fn send(&self, request: ClientRequest) -> HttpResult<ClientResponse> {
        let mut request = request;

        for _ in 0..=self.config.max_redirects {
            let mut conn = Connection::connect(&request.url, &self.config)?;
            let response = conn.request(&request, &self.config)?;

            if !response.is_redirect() {
                return Ok(response);
            }
            let Some(location) = response.headers.get("Location") else {
                // A redirect without a target is not actionable; hand it back
                // rather than guessing.
                return Ok(response);
            };

            let target = request.url.join(location.trim())?;
            request = redirect_request(request, target, response.code);
        }

        Err(HttpError::TooManyRedirects)
    }

    /// Sends a request without following redirects.
    pub fn send_once(&self, request: &ClientRequest) -> HttpResult<ClientResponse> {
        let mut conn = Connection::connect(&request.url, &self.config)?;
        conn.request(request, &self.config)
    }
}

/// Rewrites a request for the next redirect hop.
fn redirect_request(mut request: ClientRequest, target: Url, code: u16) -> ClientRequest {
    // 303 always becomes a GET; 301 and 302 do so for anything that is not
    // already GET or HEAD, which is what every browser does in practice.
    let downgrade = code == 303
        || (matches!(code, 301 | 302) && !matches!(request.method, Method::Get | Method::Head));

    if downgrade {
        request.method = Method::Get;
        request.body = Vec::new();
        request.headers.remove("Content-Type");
    }

    // Credentials must not leak to a different origin.
    if target.host != request.url.host || target.port != request.url.port {
        request.headers.remove("Authorization");
        request.headers.remove("Cookie");
    }

    request.url = target;
    request
}

/// Percent-encodes a form value, leaving only the unreserved characters.
fn form_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
