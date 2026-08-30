//! Absolute URLs, as needed by the client.
//!
//! [`super::Uri`] models a request target as it arrives at a server and
//! deliberately throws the authority away. A client needs the opposite: it
//! must know where to connect, and it must send the target *undecoded*, so
//! this type keeps the raw path and query.

use super::{HttpError, HttpResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    pub scheme: Scheme,
    pub host: String,
    pub port: u16,
    /// Raw, still percent-encoded path. Always starts with `/`.
    pub path: String,
    /// Raw query string without the `?`.
    pub query: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Http,
    Https,
}

impl Scheme {
    pub fn default_port(self) -> u16 {
        match self {
            Self::Http => 80,
            Self::Https => 443,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }
}

impl Url {
    pub fn parse(raw: &str) -> HttpResult<Self> {
        let (scheme, rest) = if let Some(rest) = raw.strip_prefix("http://") {
            (Scheme::Http, rest)
        } else if let Some(rest) = raw.strip_prefix("https://") {
            (Scheme::Https, rest)
        } else {
            return Err(HttpError::Malformed("url must start with http:// or https://"));
        };

        // The fragment never goes on the wire.
        let rest = rest.split('#').next().unwrap_or(rest);

        let (authority, target) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };

        // Userinfo is accepted syntactically but not used for auth; callers
        // that need it should set an Authorization header themselves.
        let authority = authority.rsplit('@').next().unwrap_or(authority);
        if authority.is_empty() {
            return Err(HttpError::Malformed("url without host"));
        }

        let (host, port) = split_host_port(authority, scheme)?;

        let (path, query) = match target.split_once('?') {
            Some((p, q)) => (p, Some(q.to_string())),
            None => (target, None),
        };

        Ok(Self {
            scheme,
            host,
            port,
            path: if path.is_empty() {
                "/".to_string()
            } else {
                path.to_string()
            },
            query,
        })
    }

    /// The value for the `Host` header. The port is omitted when it is the
    /// default for the scheme, as required by RFC 9110.
    pub fn host_header(&self) -> String {
        if self.port == self.scheme.default_port() {
            self.host.clone()
        } else if self.host.contains(':') {
            // Literal IPv6 addresses stay in brackets.
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    /// The origin-form request target, i.e. what follows the method on the
    /// request line.
    pub fn request_target(&self) -> String {
        match &self.query {
            Some(q) => format!("{}?{}", self.path, q),
            None => self.path.clone(),
        }
    }

    /// The address to connect to. IPv6 literals need brackets again here
    /// because `ToSocketAddrs` parses the string form.
    pub fn socket_addr(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    /// Resolves a `Location` value against this URL, which may be absolute,
    /// absolute-path or relative.
    pub fn join(&self, location: &str) -> HttpResult<Self> {
        if location.starts_with("http://") || location.starts_with("https://") {
            return Self::parse(location);
        }

        let location = location.split('#').next().unwrap_or(location);
        let (raw_path, query) = match location.split_once('?') {
            Some((p, q)) => (p, Some(q.to_string())),
            None => (location, None),
        };

        let path = if raw_path.starts_with('/') {
            raw_path.to_string()
        } else if raw_path.is_empty() {
            self.path.clone()
        } else {
            // Relative reference: replace the last segment of the base path.
            let base = match self.path.rfind('/') {
                Some(i) => &self.path[..=i],
                None => "/",
            };
            normalize_dots(&format!("{base}{raw_path}"))
        };

        Ok(Self {
            scheme: self.scheme,
            host: self.host.clone(),
            port: self.port,
            path,
            query,
        })
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}://{}{}",
            self.scheme.as_str(),
            self.host_header(),
            self.request_target()
        )
    }
}

fn split_host_port(authority: &str, scheme: Scheme) -> HttpResult<(String, u16)> {
    // IPv6 literals are bracketed, and their colons must not be mistaken for
    // a port separator.
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, after) = rest
            .split_once(']')
            .ok_or(HttpError::Malformed("unterminated IPv6 literal in url"))?;
        let port = match after.strip_prefix(':') {
            Some(p) => parse_port(p)?,
            None if after.is_empty() => scheme.default_port(),
            None => return Err(HttpError::Malformed("trailing junk after IPv6 literal")),
        };
        return Ok((host.to_string(), port));
    }

    match authority.rsplit_once(':') {
        Some((host, port)) => Ok((host.to_string(), parse_port(port)?)),
        None => Ok((authority.to_string(), scheme.default_port())),
    }
}

fn parse_port(raw: &str) -> HttpResult<u16> {
    raw.parse()
        .map_err(|_| HttpError::Malformed("invalid port in url"))
}

/// Collapses `.` and `..` segments so a redirect cannot walk above the root.
fn normalize_dots(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();

    for segment in path.split('/') {
        match segment {
            "." => {}
            ".." => {
                out.pop();
            }
            s => out.push(s),
        }
    }

    let joined = out.join("/");
    if joined.starts_with('/') {
        joined
    } else {
        format!("/{joined}")
    }
}
