//! A connection handle that can be shared between a reader and a writer.
//!
//! A `TcpStream` can be `try_clone`d, which is how the plain-HTTP server
//! lets one thread read while another writes. A TLS session cannot: there is
//! a single cipher state, and two independent halves would interleave
//! records and corrupt the sequence numbering.
//!
//! So both cases go through one type. Plain streams use real socket clones.
//! A TLS session is instead *split*: the two directions of TLS have separate
//! keys, IVs and sequence numbers and share nothing once the handshake is
//! done, so each half can own its own crypto with no lock between them.
//!
//! That split matters rather than being an optimisation. A single mutex over
//! the whole session deadlocks as soon as one thread blocks in `read`
//! holding it while another needs to `write` — which is exactly what a
//! WebSocket server does, with a reader thread parked on player input and a
//! game loop broadcasting from elsewhere.
//!
//! Writers are still shared behind a mutex *among themselves*, because
//! several threads may broadcast to the same client and a half-written frame
//! interleaved with another is corruption.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

/// The write half of a connection.
///
/// Cloning is cheap and always possible, which is what lets a server hand a
/// writer to a game loop while another thread owns the reader.
pub enum ConnWriter {
    Plain(TcpStream),
    #[cfg(feature = "tls")]
    Tls(Arc<Mutex<super::tls::TlsWriteHalf<TcpStream>>>),
}

impl ConnWriter {
    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(match self {
            Self::Plain(s) => Self::Plain(s.try_clone()?),
            #[cfg(feature = "tls")]
            Self::Tls(s) => Self::Tls(Arc::clone(s)),
        })
    }

    /// The socket underneath, for timeouts and `set_nodelay`.
    pub fn socket(&self) -> io::Result<TcpStream> {
        match self {
            Self::Plain(s) => s.try_clone(),
            #[cfg(feature = "tls")]
            Self::Tls(s) => s
                .lock()
                .map_err(|_| io::Error::other("tls session poisoned"))?
                .get_ref()
                .try_clone(),
        }
    }
}

impl Write for ConnWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Plain(s) => s.write(buf),
            #[cfg(feature = "tls")]
            Self::Tls(s) => s
                .lock()
                .map_err(|_| io::Error::other("tls session poisoned"))?
                .write(buf),
        }
    }

    /// Writes a whole message under one lock.
    ///
    /// Taking the lock per `write` call would let two threads interleave the
    /// halves of a frame, so the default `write_all` is not good enough here.
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            Self::Plain(s) => s.write_all(buf),
            #[cfg(feature = "tls")]
            Self::Tls(s) => s
                .lock()
                .map_err(|_| io::Error::other("tls session poisoned"))?
                .write_all(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Plain(s) => s.flush(),
            #[cfg(feature = "tls")]
            Self::Tls(s) => s
                .lock()
                .map_err(|_| io::Error::other("tls session poisoned"))?
                .flush(),
        }
    }
}

/// The read half of a connection.
pub enum ConnReader {
    Plain(TcpStream),
    #[cfg(feature = "tls")]
    Tls(Box<super::tls::TlsReadHalf<TcpStream>>),
}

impl Read for ConnReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(buf),
            // No lock: this half owns its decryption state outright, so a
            // blocking read never holds anything the writer needs.
            #[cfg(feature = "tls")]
            Self::Tls(s) => s.read(buf),
        }
    }
}

/// A connection the server can split into halves.
pub struct Conn {
    inner: Inner,
    /// Where the server parks its reader for the duration of a handler call.
    parked: ParkedReader,
}

enum Inner {
    Plain(TcpStream),
    #[cfg(feature = "tls")]
    Tls {
        /// Taken by the first `reader()` call. A TLS session has exactly one
        /// decryption state, so there can only ever be one reader.
        read: Mutex<Option<Box<super::tls::TlsReadHalf<TcpStream>>>>,
        write: Arc<Mutex<super::tls::TlsWriteHalf<TcpStream>>>,
        socket: TcpStream,
    },
}

/// Lets the server hand its reader back so a hijacking handler can carry on
/// with it.
///
/// This is not just bookkeeping for TLS: the server's `BufReader` may
/// already hold bytes the peer sent immediately after the request, and
/// dropping it would silently lose them.
type ParkedReader = Mutex<Option<std::io::BufReader<ConnReader>>>;

impl Conn {
    pub fn plain(stream: TcpStream) -> Self {
        Self {
            inner: Inner::Plain(stream),
            parked: Mutex::new(None),
        }
    }

    /// Wraps a completed TLS session, splitting it so the two directions can
    /// be driven from different threads.
    #[cfg(feature = "tls")]
    pub fn tls(session: super::tls::TlsStream<TcpStream>) -> io::Result<Self> {
        let socket = session.get_ref().try_clone()?;
        let (read, write) = session.split(|s| s.try_clone())?;

        Ok(Self {
            inner: Inner::Tls {
                read: Mutex::new(Some(Box::new(read))),
                write: Arc::new(Mutex::new(write)),
                socket,
            },
            parked: Mutex::new(None),
        })
    }

    pub fn is_tls(&self) -> bool {
        match &self.inner {
            Inner::Plain(_) => false,
            #[cfg(feature = "tls")]
            Inner::Tls { .. } => true,
        }
    }

    /// The underlying socket, for timeouts and peer addresses.
    pub fn socket(&self) -> io::Result<TcpStream> {
        match &self.inner {
            Inner::Plain(s) => s.try_clone(),
            #[cfg(feature = "tls")]
            Inner::Tls { socket, .. } => socket.try_clone(),
        }
    }

    /// Parks a reader so a handler can reclaim it. Called by the server
    /// around `Handler::handle`.
    pub(crate) fn park_reader(&self, reader: std::io::BufReader<ConnReader>) {
        if let Ok(mut slot) = self.parked.lock() {
            *slot = Some(reader);
        }
    }

    pub(crate) fn unpark_reader(&self) -> Option<std::io::BufReader<ConnReader>> {
        self.parked.lock().ok().and_then(|mut slot| slot.take())
    }

    /// Takes the buffered reader the server was using, for a handler that
    /// hijacks the connection.
    ///
    /// Preferred over [`Self::reader`] inside a handler: it preserves any
    /// bytes already buffered past the request, which a fresh reader would
    /// drop. Returns `None` outside a handler, or if it was already taken.
    pub fn take_buffered_reader(&self) -> Option<std::io::BufReader<ConnReader>> {
        self.unpark_reader()
    }

    /// The read half.
    ///
    /// For TLS this succeeds exactly once: the session has a single
    /// decryption state and handing out two readers would desynchronise it.
    /// Inside a handler prefer [`Self::take_buffered_reader`].
    pub fn reader(&self) -> io::Result<ConnReader> {
        Ok(match &self.inner {
            Inner::Plain(s) => ConnReader::Plain(s.try_clone()?),
            #[cfg(feature = "tls")]
            Inner::Tls { read, .. } => read
                .lock()
                .map_err(|_| io::Error::other("tls session poisoned"))?
                .take()
                .map(ConnReader::Tls)
                .ok_or_else(|| io::Error::other("tls read half already taken"))?,
        })
    }

    /// The write half. Always cloneable, so a broadcast loop can hold one
    /// per client.
    pub fn writer(&self) -> io::Result<ConnWriter> {
        Ok(match &self.inner {
            Inner::Plain(s) => ConnWriter::Plain(s.try_clone()?),
            #[cfg(feature = "tls")]
            Inner::Tls { write, .. } => ConnWriter::Tls(Arc::clone(write)),
        })
    }
}
