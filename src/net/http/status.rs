#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Status {
    Ok = 200,
    Created = 201,
    NoContent = 204,
    PartialContent = 206,
    MovedPermanently = 301,
    Found = 302,
    NotModified = 304,
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    PayloadTooLarge = 413,
    UriTooLong = 414,
    RangeNotSatisfiable = 416,
    InternalServerError = 500,
    NotImplemented = 501,
    ServiceUnavailable = 503,
    HttpVersionNotSupported = 505,
    SwitchingProtocols = 101,
}

impl Status {
    pub fn code(&self) -> u16 {
        *self as u16
    }

    pub fn reason(&self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Created => "Created",
            Self::NoContent => "No Content",
            Self::PartialContent => "Partial Content",
            Self::MovedPermanently => "Moved Permanently",
            Self::Found => "Found",
            Self::NotModified => "Not Modified",
            Self::BadRequest => "Bad Request",
            Self::Unauthorized => "Unauthorized",
            Self::Forbidden => "Forbidden",
            Self::NotFound => "Not Found",
            Self::MethodNotAllowed => "Method Not Allowed",
            Self::PayloadTooLarge => "Payload Too Large",
            Self::UriTooLong => "URI Too Long",
            Self::RangeNotSatisfiable => "Range Not Satisfiable",
            Self::InternalServerError => "Internal Server Error",
            Self::NotImplemented => "Not Implemented",
            Self::ServiceUnavailable => "Service Unavailable",
            Self::HttpVersionNotSupported => "HTTP Version Not Supported",
            Self::SwitchingProtocols => "Switching Protocols",
        }
    }

    /// Per RFC 9110 these statuses must never carry a body.
    pub fn allows_body(&self) -> bool {
        !matches!(self, Self::NoContent | Self::NotModified | Self::SwitchingProtocols)
    }
}
