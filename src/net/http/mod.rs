//! HTTP/1.1 server and client.
//!
//! No TLS. For HTTPS the stream must later be wrapped in a TLS layer; the
//! server types are deliberately built around `TcpStream` and would need to
//! be made generic over `Read + Write` for that. The client already parses
//! through `BufRead`, so only its connection setup is in the way.

mod client;
mod client_response;
mod conn;
mod error;
mod files;
mod headers;
mod io;
mod method;
pub mod mime;
mod request;
mod response;
mod server;
mod status;
#[cfg(feature = "tls")]
pub mod tls;
mod transport;
mod uri;
mod url;
pub mod ws;

pub use client::{Client, ClientConfig, ClientRequest, Connection};
pub use conn::{Conn, ConnReader, ConnWriter};
pub use client_response::ClientResponse;
pub use error::{HttpError, HttpResult};
pub use files::StaticFiles;
pub use headers::Headers;
pub use method::Method;
pub use request::{Limits, Request};
pub use response::Response;
pub use server::{Action, Handler, Server, ServerConfig, SimpleHandler, serve_connection, simple};
pub use status::Status;
#[cfg(feature = "tls")]
pub use tls::{ServerTlsConfig, TlsConfig, TlsError, TlsStream, Verifier};
pub use transport::Transport;
pub use uri::{Uri, percent_decode};
pub use url::{Scheme, Url};
pub use ws::{Message, WsError, WsReader, WsWriter};

/// Sends a one-off GET with the default configuration.
pub fn get(url: &str) -> HttpResult<ClientResponse> {
    Client::new().get(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_query_pairs() {
        let uri = Uri::parse("/search?q=hello+world&lang=de").unwrap();
        assert_eq!(uri.path, "/search");
        assert_eq!(uri.query_param("q").as_deref(), Some("hello world"));
        assert_eq!(uri.query_param("lang").as_deref(), Some("de"));
    }

    #[test]
    fn decodes_percent_escapes_in_path() {
        let uri = Uri::parse("/my%20file.txt").unwrap();
        assert_eq!(uri.path, "/my file.txt");
    }

    #[test]
    fn rejects_bad_percent_escapes() {
        assert!(Uri::parse("/%zz").is_err());
        assert!(Uri::parse("/%4").is_err());
    }

    #[test]
    fn strips_absolute_form() {
        let uri = Uri::parse("http://example.com/a/b?x=1").unwrap();
        assert_eq!(uri.path, "/a/b");
        assert_eq!(uri.query.as_deref(), Some("x=1"));
    }

    #[test]
    fn header_lookup_is_case_insensitive() {
        let mut h = Headers::new();
        h.append("Content-Type", "text/html");
        assert_eq!(h.get("content-type"), Some("text/html"));
        assert_eq!(h.get("CONTENT-TYPE"), Some("text/html"));
    }

    #[test]
    fn finds_token_in_comma_list() {
        let mut h = Headers::new();
        h.append("Connection", "keep-alive, Upgrade");
        assert!(h.contains_token("Connection", "upgrade"));
        assert!(h.contains_token("connection", "KEEP-ALIVE"));
        assert!(!h.contains_token("Connection", "close"));
    }

    #[test]
    fn set_replaces_existing() {
        let mut h = Headers::new();
        h.append("X", "1");
        h.append("x", "2");
        h.set("X", "3");
        assert_eq!(h.get_all("x").count(), 1);
        assert_eq!(h.get("x"), Some("3"));
    }

    #[test]
    fn response_has_content_length_and_no_stray_charset() {
        let res = Response::html("<h1>hi</h1>");
        let mut out = Vec::new();
        res.write_to(&mut out, true, true).unwrap();
        let text = String::from_utf8(out).unwrap();

        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(text.contains("Content-Type: text/html; charset=utf-8\r\n"));
        assert!(text.contains("Content-Length: 11\r\n"));
        assert!(text.ends_with("\r\n\r\n<h1>hi</h1>"));
    }

    #[test]
    fn head_omits_body_but_keeps_length() {
        let res = Response::text("hello");
        let mut out = Vec::new();
        res.write_to(&mut out, false, false).unwrap();
        let text = String::from_utf8(out).unwrap();

        assert!(text.contains("Content-Length: 5\r\n"));
        assert!(text.ends_with("\r\n\r\n"));
        assert!(!text.contains("hello"));
    }

    #[test]
    fn traversal_attempts_do_not_escape_root() {
        let root = std::env::temp_dir().join("iron_oxide_http_test_root");
        let _ = std::fs::create_dir_all(&root);
        std::fs::write(root.join("ok.txt"), b"inside").unwrap();

        let files = StaticFiles::new(&root);
        let serve = |path: &str| {
            let uri = Uri::parse(path).unwrap();
            let req = Request {
                method: Method::Get,
                uri,
                version_minor: 1,
                headers: Headers::new(),
                body: Vec::new(),
            };
            files.serve(&req).status
        };

        assert_eq!(serve("/ok.txt"), Status::Ok);
        assert_eq!(serve("/../../../../etc/passwd"), Status::NotFound);
        assert_eq!(serve("/%2e%2e%2f%2e%2e%2fetc%2fpasswd"), Status::NotFound);
        assert_eq!(serve("/./ok.txt"), Status::Ok);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn parses_url_parts() {
        let url = Url::parse("http://example.com/a/b?x=1").unwrap();
        assert_eq!(url.scheme, Scheme::Http);
        assert_eq!(url.host, "example.com");
        assert_eq!(url.port, 80);
        assert_eq!(url.path, "/a/b");
        assert_eq!(url.query.as_deref(), Some("x=1"));
        assert_eq!(url.request_target(), "/a/b?x=1");
    }

    #[test]
    fn url_keeps_path_encoded() {
        // The server decodes; the client must not, or the target changes.
        let url = Url::parse("http://example.com/my%20file.txt").unwrap();
        assert_eq!(url.path, "/my%20file.txt");
    }

    #[test]
    fn host_header_omits_default_port() {
        assert_eq!(
            Url::parse("http://example.com/").unwrap().host_header(),
            "example.com"
        );
        assert_eq!(
            Url::parse("http://example.com:8080/").unwrap().host_header(),
            "example.com:8080"
        );
    }

    #[test]
    fn parses_ipv6_authority() {
        let url = Url::parse("http://[::1]:8080/x").unwrap();
        assert_eq!(url.host, "::1");
        assert_eq!(url.port, 8080);
        assert_eq!(url.socket_addr(), "[::1]:8080");
    }

    #[test]
    fn rejects_url_without_scheme() {
        assert!(Url::parse("example.com/x").is_err());
        assert!(Url::parse("http://").is_err());
    }

    #[test]
    fn joins_relative_locations() {
        let base = Url::parse("http://example.com/a/b?x=1").unwrap();

        assert_eq!(base.join("/c").unwrap().path, "/c");
        assert_eq!(base.join("c").unwrap().path, "/a/c");
        assert_eq!(base.join("../c").unwrap().path, "/c");
        assert_eq!(
            base.join("http://other.com/d").unwrap().host,
            "other.com"
        );
        // The query does not carry over to a new target.
        assert_eq!(base.join("/c").unwrap().query, None);
    }

    #[test]
    fn https_without_trust_anchors_is_refused() {
        // The default verifier trusts nothing, so a connection must fail
        // rather than silently accept whatever certificate turns up. The
        // handshake never gets that far here, but the config is what matters.
        #[cfg(feature = "tls")]
        {
            let config = TlsConfig::default();
            let err = config
                .verifier
                .verify(&[], "example.com", 0)
                .unwrap_err();
            assert!(
                matches!(err, tls::TlsError::BadCertificate(_)),
                "got {err:?}"
            );
        }
    }

    #[cfg(feature = "tls")]
    #[test]
    fn https_scheme_reaches_the_tls_path() {
        // Nothing is listening, so this must fail at connect rather than by
        // falling back to plaintext.
        let err = Client::new().get("https://127.0.0.1:1/").unwrap_err();
        assert!(matches!(err, HttpError::Io(_)), "got {err:?}");
    }

    /// Runs a server on a random port and returns its address.
    fn spawn_test_server(handler: impl Handler) -> std::net::SocketAddr {
        let server = Server::bind("127.0.0.1:0").unwrap();
        let addr = server.local_addr().unwrap();
        std::thread::spawn(move || {
            let _ = server.run(handler);
        });
        addr
    }

    #[test]
    fn client_and_server_round_trip() {
        let addr = spawn_test_server(simple(|req: &Request| match req.path() {
            "/hello" => Response::text("hi there"),
            "/echo" => Response::text(req.body.clone()),
            _ => Response::error(Status::NotFound),
        }));

        let client = Client::new();

        let res = client.get(&format!("http://{addr}/hello")).unwrap();
        assert_eq!(res.code, 200);
        assert_eq!(res.status(), Some(Status::Ok));
        assert_eq!(res.text(), "hi there");

        let res = client
            .post(&format!("http://{addr}/echo"), "text/plain", "ping")
            .unwrap();
        assert_eq!(res.text(), "ping");

        let res = client.get(&format!("http://{addr}/missing")).unwrap();
        assert_eq!(res.code, 404);
        assert!(!res.is_success());
    }

    #[test]
    fn head_response_has_no_body() {
        let addr = spawn_test_server(simple(|_: &Request| Response::text("hello")));

        let res = Client::new().head(&format!("http://{addr}/")).unwrap();
        assert_eq!(res.code, 200);
        assert_eq!(res.headers.get("Content-Length"), Some("5"));
        assert!(res.body.is_empty());
    }

    #[test]
    fn follows_redirect_to_final_target() {
        let addr = spawn_test_server(simple(|req: &Request| match req.path() {
            "/start" => Response::redirect(Status::Found, "/end"),
            "/end" => Response::text("arrived"),
            _ => Response::error(Status::NotFound),
        }));

        let res = Client::new().get(&format!("http://{addr}/start")).unwrap();
        assert_eq!(res.code, 200);
        assert_eq!(res.text(), "arrived");
    }

    #[test]
    fn send_once_does_not_follow_redirects() {
        let addr = spawn_test_server(simple(|_: &Request| {
            Response::redirect(Status::Found, "/end")
        }));

        let req = ClientRequest::parse(Method::Get, &format!("http://{addr}/start")).unwrap();
        let res = Client::new().send_once(&req).unwrap();
        assert_eq!(res.code, 302);
        assert_eq!(res.headers.get("Location"), Some("/end"));
    }

    #[test]
    fn redirect_loop_gives_up() {
        let addr = spawn_test_server(simple(|_: &Request| {
            Response::redirect(Status::Found, "/loop")
        }));

        let err = Client::new()
            .get(&format!("http://{addr}/loop"))
            .unwrap_err();
        assert!(matches!(err, HttpError::TooManyRedirects), "got {err:?}");
    }

    #[test]
    fn post_becomes_get_after_302() {
        let addr = spawn_test_server(simple(|req: &Request| match req.path() {
            "/submit" => Response::redirect(Status::Found, "/done"),
            "/done" => Response::text(req.method.as_str()),
            _ => Response::error(Status::NotFound),
        }));

        let res = Client::new()
            .post(&format!("http://{addr}/submit"), "text/plain", "data")
            .unwrap();
        assert_eq!(res.text(), "GET");
    }

    #[test]
    fn error_for_status_reports_the_code() {
        let addr = spawn_test_server(simple(|_: &Request| Response::error(Status::NotFound)));

        let err = Client::new()
            .get(&format!("http://{addr}/x"))
            .unwrap()
            .error_for_status()
            .unwrap_err();
        assert!(matches!(err, HttpError::Status { code: 404, .. }), "got {err:?}");
    }

    #[test]
    fn sends_host_and_user_agent() {
        let addr = spawn_test_server(simple(|req: &Request| {
            let host = req.headers.get("Host").unwrap_or("").to_string();
            let ua = req.headers.get("User-Agent").unwrap_or("").to_string();
            Response::text(format!("{host}|{ua}"))
        }));

        let res = Client::new().get(&format!("http://{addr}/")).unwrap();
        let text = res.text();
        let (host, ua) = text.split_once('|').unwrap();
        assert_eq!(host, addr.to_string());
        assert!(ua.starts_with("iron_oxide/"), "got {ua:?}");
    }

    #[test]
    fn form_body_is_encoded() {
        let addr = spawn_test_server(simple(|req: &Request| {
            let ct = req.headers.get("Content-Type").unwrap_or("").to_string();
            Response::text(format!("{ct}|{}", req.body_as_str().unwrap_or("")))
        }));

        let req = ClientRequest::parse(Method::Post, &format!("http://{addr}/"))
            .unwrap()
            .form([("q", "hello world"), ("lang", "de&fr")]);
        let res = Client::new().send(req).unwrap();

        assert_eq!(
            res.text(),
            "application/x-www-form-urlencoded|q=hello+world&lang=de%26fr"
        );
    }

    #[test]
    fn reads_chunked_response_body() {
        // The crate's own server never chunks, so this speaks raw HTTP.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
            // Drain the request head so the client's write cannot block.
            loop {
                let mut line = String::new();
                std::io::BufRead::read_line(&mut reader, &mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
            }
            std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n\
                  5\r\nhello\r\n2\r\n, \r\n5\r\nworld\r\n0\r\n\r\n",
            )
            .unwrap();
        });

        let res = Client::new().get(&format!("http://{addr}/")).unwrap();
        assert_eq!(res.text(), "hello, world");
    }

    #[test]
    fn reads_body_delimited_by_eof() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                std::io::BufRead::read_line(&mut reader, &mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
            }
            // No Content-Length and no chunking: the close is the framing.
            std::io::Write::write_all(&mut stream, b"HTTP/1.0 200 OK\r\n\r\nuntil eof").unwrap();
        });

        let res = Client::new().get(&format!("http://{addr}/")).unwrap();
        assert_eq!(res.text(), "until eof");
        assert!(!res.keeps_alive());
    }
}
