use std::io::BufReader;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use super::conn::Conn;
use super::{HttpError, Limits, Request, Response, Status};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub limits: Limits,
    /// How long to wait for the next request on an open connection before
    /// closing it.
    pub keep_alive_timeout: Option<Duration>,
    /// How long writing a response may take.
    pub write_timeout: Option<Duration>,
    /// How many requests may share a single connection.
    pub max_requests_per_connection: usize,
    /// Serve HTTPS with this identity. `None` serves plain HTTP.
    ///
    /// Shared rather than cloned: it holds a private key, and every
    /// connection needs the same one.
    #[cfg(feature = "tls")]
    pub tls: Option<Arc<super::tls::ServerTlsConfig>>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            limits: Limits::default(),
            keep_alive_timeout: Some(Duration::from_secs(30)),
            write_timeout: Some(Duration::from_secs(30)),
            max_requests_per_connection: 100,
            #[cfg(feature = "tls")]
            tls: None,
        }
    }
}

/// What happens to the connection after the response.
pub enum Action {
    /// Send the response, then continue per keep-alive rules.
    Respond(Response),
    /// Send the response and close the connection afterwards.
    RespondAndClose(Response),
    /// The handler takes over the socket itself, e.g. for a WebSocket
    /// upgrade. The server does not touch it afterwards.
    Hijack,
}

impl From<Response> for Action {
    fn from(res: Response) -> Self {
        Action::Respond(res)
    }
}

/// Handles a request.
///
/// `conn` is the same connection the request was read from and is only
/// needed for `Action::Hijack`. It is a [`Conn`] rather than a `TcpStream`
/// so that a hijacking handler works identically over TLS.
pub trait Handler: Send + Sync + 'static {
    fn handle(&self, req: &Request, conn: &Conn, peer: SocketAddr) -> Action;
}

/// Lets a plain `Fn(&Request) -> Response` be used as a handler.
///
/// Wrapped in a struct rather than implemented on `F` directly, because a
/// blanket impl here would collide with the one for `Action`-returning
/// closures. Use [`simple`] to build it.
pub struct SimpleHandler<F>(F);

/// Wraps a closure that only ever returns a response.
pub fn simple<F>(f: F) -> SimpleHandler<F>
where
    F: Fn(&Request) -> Response + Send + Sync + 'static,
{
    SimpleHandler(f)
}

impl<F> Handler for SimpleHandler<F>
where
    F: Fn(&Request) -> Response + Send + Sync + 'static,
{
    fn handle(&self, req: &Request, _conn: &Conn, _peer: SocketAddr) -> Action {
        Action::Respond((self.0)(req))
    }
}

/// Lets a closure with the full signature act as a handler, so that upgrades
/// and other connection takeovers are reachable without a custom type.
impl<F> Handler for F
where
    F: Fn(&Request, &Conn, SocketAddr) -> Action + Send + Sync + 'static,
{
    fn handle(&self, req: &Request, conn: &Conn, peer: SocketAddr) -> Action {
        self(req, conn, peer)
    }
}

pub struct Server {
    listener: TcpListener,
    config: ServerConfig,
}

impl Server {
    pub fn bind(addr: impl ToSocketAddrs) -> std::io::Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(addr)?,
            config: ServerConfig::default(),
        })
    }

    pub fn with_config(mut self, config: ServerConfig) -> Self {
        self.config = config;
        self
    }

    /// Serves HTTPS with the given identity.
    ///
    /// Load one with [`super::ServerTlsConfig::from_pem_files`].
    #[cfg(feature = "tls")]
    pub fn with_tls(mut self, tls: super::tls::ServerTlsConfig) -> Self {
        self.config.tls = Some(Arc::new(tls));
        self
    }

    /// Whether this server is serving HTTPS.
    pub fn is_tls(&self) -> bool {
        #[cfg(feature = "tls")]
        {
            self.config.tls.is_some()
        }
        #[cfg(not(feature = "tls"))]
        {
            false
        }
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Accepts connections and serves each one on its own thread.
    ///
    /// Blocks until the listener dies. One thread per connection does not
    /// scale to tens of thousands of clients, but without async it is the
    /// simplest option that does not block.
    pub fn run(self, handler: impl Handler) -> std::io::Result<()> {
        let handler = Arc::new(handler);

        for stream in self.listener.incoming() {
            let stream = match stream {
                Ok(s) => s,
                // A single failed accept must not kill the server, e.g. when
                // the client vanishes immediately.
                Err(_) => continue,
            };

            let handler = Arc::clone(&handler);
            let config = self.config.clone();

            std::thread::spawn(move || {
                serve_connection(stream, &config, handler.as_ref());
            });
        }

        Ok(())
    }

    /// Like `run`, but serves everything on the calling thread. Useful for
    /// tests and simple cases.
    pub fn run_single_threaded(self, handler: impl Handler) -> std::io::Result<()> {
        for stream in self.listener.incoming() {
            let Ok(stream) = stream else { continue };
            serve_connection(stream, &self.config, &handler);
        }
        Ok(())
    }
}

/// Serves all requests on one connection, one after another.
pub fn serve_connection(stream: TcpStream, config: &ServerConfig, handler: &impl Handler) {
    let Ok(peer) = stream.peer_addr() else { return };

    let _ = stream.set_nodelay(true);
    let _ = stream.set_write_timeout(config.write_timeout);
    let _ = stream.set_read_timeout(config.keep_alive_timeout);

    // The TLS handshake happens before a single byte of HTTP is read, so a
    // failure here means no request was ever seen and there is nothing to
    // answer with.
    let conn = match accept_tls(stream, config) {
        Ok(conn) => conn,
        Err(_) => return,
    };

    let (Ok(reader), Ok(mut writes)) = (conn.reader(), conn.writer()) else {
        return;
    };
    let mut reader = BufReader::with_capacity(8 * 1024, reader);

    for _ in 0..config.max_requests_per_connection {
        let request = match Request::read_from(&mut reader, &config.limits) {
            Ok(req) => req,
            Err(HttpError::ConnectionClosed) => return,
            Err(HttpError::Io(_)) => return,
            Err(e) => {
                // Broken request: answer once, then close. Reading on from a
                // desynchronized connection would be unsafe.
                let _ = Response::error(e.status()).write_to(&mut writes, true, false);
                return;
            }
        };

        let keep_alive = request.wants_keep_alive();
        let include_body = request.method.expects_body_in_response();

        // Lend the reader to the handler for the duration of the call. A
        // hijacking handler takes it — along with anything already buffered
        // past this request — and whatever is left comes back afterwards.
        conn.park_reader(reader);
        let action = handler.handle(&request, &conn, peer);

        match action {
            Action::Hijack => return,
            Action::RespondAndClose(res) => {
                let _ = res.write_to(&mut writes, include_body, false);
                return;
            }
            Action::Respond(res) => {
                if res.write_to(&mut writes, include_body, keep_alive).is_err() || !keep_alive {
                    return;
                }
            }
        }

        // Take the reader back for the next request on this connection.
        let Some(returned) = conn.unpark_reader() else {
            return;
        };
        reader = returned;
    }

    // Limit reached: close politely with a hint.
    let _ = Response::error(Status::ServiceUnavailable).write_to(&mut writes, true, false);
}

#[cfg(feature = "tls")]
fn accept_tls(stream: TcpStream, config: &ServerConfig) -> Result<Conn, ()> {
    match &config.tls {
        Some(tls) => {
            let session = super::tls::TlsStream::accept(stream, tls).map_err(|_| ())?;
            Conn::tls(session).map_err(|_| ())
        }
        None => Ok(Conn::plain(stream)),
    }
}

#[cfg(not(feature = "tls"))]
fn accept_tls(stream: TcpStream, _config: &ServerConfig) -> Result<Conn, ()> {
    Ok(Conn::plain(stream))
}
