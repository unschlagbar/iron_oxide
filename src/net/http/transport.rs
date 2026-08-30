//! The byte stream underneath HTTP.
//!
//! HTTP does not care whether it runs on a bare socket or through TLS, but
//! the two differ in one way that leaks upward: a `TcpStream` can be
//! `try_clone`d into independent read and write halves, while a TLS session
//! cannot. There is one cipher state, and two handles writing into it would
//! corrupt the record sequence.
//!
//! So the connection owns a single `Transport` and does both directions
//! through it, rather than splitting.

use std::io::{self, Read, Write};
use std::net::TcpStream;

pub enum Transport {
    Plain(TcpStream),
    #[cfg(feature = "tls")]
    Tls(Box<super::tls::TlsStream<TcpStream>>),
}

impl Transport {
    /// The socket underneath, for timeouts and peer addresses.
    pub fn socket(&self) -> &TcpStream {
        match self {
            Self::Plain(s) => s,
            #[cfg(feature = "tls")]
            Self::Tls(s) => s.get_ref(),
        }
    }

    pub fn is_tls(&self) -> bool {
        match self {
            Self::Plain(_) => false,
            #[cfg(feature = "tls")]
            Self::Tls(_) => true,
        }
    }
}

impl Read for Transport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(buf),
            #[cfg(feature = "tls")]
            Self::Tls(s) => s.read(buf),
        }
    }
}

impl Write for Transport {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Plain(s) => s.write(buf),
            #[cfg(feature = "tls")]
            Self::Tls(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Plain(s) => s.flush(),
            #[cfg(feature = "tls")]
            Self::Tls(s) => s.flush(),
        }
    }
}
