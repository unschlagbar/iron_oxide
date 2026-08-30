use std::fmt;

/// An error while reading or parsing a request.
///
/// Every error knows the status it should be answered with, so the server
/// never has to guess how to react to a broken request.
#[derive(Debug)]
pub enum HttpError {
    /// The connection was closed before anything was read. With keep-alive
    /// this is the normal case, not a real error.
    ConnectionClosed,
    /// The request line or a header was syntactically broken.
    Malformed(&'static str),
    /// A well-formed method that this server does not implement.
    UnsupportedMethod,
    /// The header block or body exceeded the configured limits.
    TooLarge(&'static str),
    /// The HTTP version is not supported.
    UnsupportedVersion,
    /// Client side: the peer answered with a status the caller treats as an
    /// error. Never produced by the server.
    Status { code: u16, reason: String },
    /// Client side: too many redirects were followed.
    TooManyRedirects,
    Io(std::io::Error),
}

impl HttpError {
    /// The status this error should be answered with.
    pub fn status(&self) -> super::Status {
        use super::Status;
        match self {
            Self::ConnectionClosed | Self::Io(_) => Status::BadRequest,
            Self::Malformed(_) => Status::BadRequest,
            Self::UnsupportedMethod => Status::NotImplemented,
            Self::TooLarge(_) => Status::PayloadTooLarge,
            Self::UnsupportedVersion => Status::HttpVersionNotSupported,
            // Client-side variants have no meaningful server answer; a
            // failed outgoing request is this server's own problem.
            Self::Status { .. } | Self::TooManyRedirects => Status::InternalServerError,
        }
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionClosed => write!(f, "connection closed by peer"),
            Self::Malformed(what) => write!(f, "malformed request: {what}"),
            Self::UnsupportedMethod => write!(f, "unsupported method"),
            Self::TooLarge(what) => write!(f, "too large: {what}"),
            Self::UnsupportedVersion => write!(f, "unsupported HTTP version"),
            Self::Status { code, reason } => write!(f, "server responded {code} {reason}"),
            Self::TooManyRedirects => write!(f, "too many redirects"),
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for HttpError {}

impl From<std::io::Error> for HttpError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// A TLS failure surfaces as an io error, since that is where it stops the
/// request. The message is preserved so the cause stays visible.
#[cfg(feature = "tls")]
impl From<super::tls::TlsError> for HttpError {
    fn from(e: super::tls::TlsError) -> Self {
        Self::Io(e.into())
    }
}

pub type HttpResult<T> = Result<T, HttpError>;
