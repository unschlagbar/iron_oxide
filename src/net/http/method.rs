use super::{HttpError, HttpResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Patch,
    Options,
}

impl Method {
    pub fn parse(raw: &[u8]) -> HttpResult<Self> {
        // Methods are case-sensitive, so a byte comparison is enough.
        Ok(match raw {
            b"GET" => Self::Get,
            b"HEAD" => Self::Head,
            b"POST" => Self::Post,
            b"PUT" => Self::Put,
            b"DELETE" => Self::Delete,
            b"PATCH" => Self::Patch,
            b"OPTIONS" => Self::Options,
            _ => return Err(HttpError::UnsupportedMethod),
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
            Self::Options => "OPTIONS",
        }
    }

    /// HEAD is answered with headers only; the body is dropped.
    pub fn expects_body_in_response(&self) -> bool {
        !matches!(self, Self::Head)
    }
}
