//! Server-side TLS, tested from both directions.
//!
//! Our client talking to our server proves only that the two agree with each
//! other, so the important test here is the one that puts `openssl s_client`
//! on the other end.

#![cfg(feature = "tls")]

use std::io::{Read, Write};
use std::process::{Command, Stdio};

use iron_oxide::net::http::{
    Client, ClientConfig, Response, Server, ServerTlsConfig, TlsConfig, Verifier, simple,
};

fn have_openssl() -> bool {
    Command::new("openssl")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Generates a self-signed PKCS#8 certificate/key pair for `localhost`.
fn make_identity(dir: &std::path::Path) -> Option<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    let der = dir.join("cert.der");

    let status = Command::new("openssl")
        .args(["req", "-x509", "-newkey", "rsa:2048", "-keyout"])
        .arg(&key)
        .arg("-out")
        .arg(&cert)
        .args(["-days", "3650", "-nodes", "-subj", "/CN=localhost"])
        .arg("-addext")
        .arg("subjectAltName=DNS:localhost,IP:127.0.0.1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }

    Command::new("openssl")
        .arg("x509")
        .arg("-in")
        .arg(&cert)
        .args(["-outform", "DER", "-out"])
        .arg(&der)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;

    Some((
        std::fs::read(&cert).ok()?,
        std::fs::read(&key).ok()?,
        std::fs::read(&der).ok()?,
    ))
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("iron_oxide_tlssrv_{name}"));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Starts an HTTPS server on a random port and returns its address.
fn spawn_https(cert_pem: &[u8], key_pem: &[u8]) -> std::net::SocketAddr {
    let tls = ServerTlsConfig::from_pem(cert_pem, key_pem).expect("identity should load");
    let server = Server::bind("127.0.0.1:0").unwrap().with_tls(tls);
    let addr = server.local_addr().unwrap();

    std::thread::spawn(move || {
        let _ = server.run(simple(|req: &iron_oxide::net::http::Request| {
            Response::text(format!("secure:{}", req.path()))
        }));
    });

    // Give the listener a moment to be ready.
    std::thread::sleep(std::time::Duration::from_millis(200));
    addr
}

#[test]
fn our_client_talks_to_our_server() {
    if !have_openssl() {
        return;
    }
    let dir = scratch("roundtrip");
    let Some((cert_pem, key_pem, cert_der)) = make_identity(&dir) else {
        return;
    };

    let addr = spawn_https(&cert_pem, &key_pem);

    let client = Client::with_config(ClientConfig {
        tls: TlsConfig {
            verifier: Verifier::Pinned(vec![cert_der]),
            alpn: vec!["http/1.1".to_string()],
        },
        ..Default::default()
    });

    let res = client
        .get(&format!("https://localhost:{}/hello", addr.port()))
        .expect("https request should succeed");

    assert_eq!(res.code, 200);
    assert_eq!(res.text(), "secure:/hello");
}

#[test]
fn openssl_client_completes_our_handshake() {
    if !have_openssl() {
        return;
    }
    let dir = scratch("openssl_client");
    let Some((cert_pem, key_pem, _)) = make_identity(&dir) else {
        return;
    };

    let addr = spawn_https(&cert_pem, &key_pem);

    // The independent check: OpenSSL, not our own code, drives this
    // handshake and reports whether the protocol was followed.
    let mut child = Command::new("openssl")
        .args([
            "s_client",
            "-connect",
            &format!("127.0.0.1:{}", addr.port()),
            "-servername",
            "localhost",
            "-tls1_3",
            "-CAfile",
            dir.join("cert.pem").to_str().unwrap(),
            "-verify_return_error",
            "-quiet",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("s_client should start");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"GET /from-openssl HTTP/1.0\r\n\r\n")
        .unwrap();

    let output = child.wait_with_output().expect("s_client should finish");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("secure:/from-openssl"),
        "openssl did not get a response.\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn client_rejects_our_server_when_the_pin_is_wrong() {
    if !have_openssl() {
        return;
    }
    let dir = scratch("wrongpin");
    let Some((cert_pem, key_pem, _)) = make_identity(&dir) else {
        return;
    };

    let other = scratch("wrongpin_other");
    let Some((_, _, other_der)) = make_identity(&other) else {
        return;
    };

    let addr = spawn_https(&cert_pem, &key_pem);

    let client = Client::with_config(ClientConfig {
        tls: TlsConfig {
            verifier: Verifier::Pinned(vec![other_der]),
            alpn: vec!["http/1.1".to_string()],
        },
        ..Default::default()
    });

    // A genuine server, but not the one we pinned: must not connect.
    // `expect_err` is unavailable because the success type is not `Debug`.
    #[allow(clippy::err_expect)]
    let err = client
        .get(&format!("https://localhost:{}/", addr.port()))
        .err()
        .expect("a mismatched pin must fail");

    let text = err.to_string();
    assert!(text.contains("certificate"), "unexpected error: {text}");
}

/// A hijacked TLS connection must support reading and writing from two
/// threads at once.
///
/// This is the shape a WebSocket server has: one thread parked in `read`
/// waiting for the peer, another pushing updates. An implementation that
/// puts a single lock around the whole session deadlocks here, and the
/// symptom is a connection that completes its handshake and then goes
/// silent, so it is worth a test of its own.
#[test]
fn hijacked_tls_connection_reads_and_writes_concurrently() {
    if !have_openssl() {
        return;
    }
    let dir = scratch("concurrent");
    let Some((cert_pem, key_pem, cert_der)) = make_identity(&dir) else {
        return;
    };

    use iron_oxide::net::http::{Action, Conn, Request};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let tls = ServerTlsConfig::from_pem(&cert_pem, &key_pem).unwrap();
    let server = Server::bind("127.0.0.1:0").unwrap().with_tls(tls);
    let addr = server.local_addr().unwrap();

    let pushed = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&pushed);

    std::thread::spawn(move || {
        let _ = server.run(move |_req: &Request, conn: &Conn, _peer| {
            let Ok(mut writer) = conn.writer() else {
                return Action::Hijack;
            };
            let Some(mut reader) = conn.take_buffered_reader() else {
                return Action::Hijack;
            };
            let flag = Arc::clone(&flag);

            // One thread blocks on the peer; another writes meanwhile. If
            // the two share a lock, the write never happens.
            std::thread::spawn(move || {
                std::thread::spawn(move || {
                    let mut byte = [0u8; 1];
                    let _ = std::io::Read::read(&mut reader, &mut byte);
                });

                std::thread::sleep(std::time::Duration::from_millis(300));
                if std::io::Write::write_all(&mut writer, b"pushed\n").is_ok() {
                    flag.store(true, Ordering::SeqCst);
                }
            });

            Action::Hijack
        });
    });
    std::thread::sleep(std::time::Duration::from_millis(200));

    let tcp = std::net::TcpStream::connect(addr).unwrap();
    tcp.set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .unwrap();
    let config = TlsConfig {
        verifier: Verifier::Pinned(vec![cert_der]),
        alpn: Vec::new(),
    };
    let mut tls = iron_oxide::net::http::TlsStream::connect(tcp, "localhost", &config).unwrap();

    tls.write_all(b"GET /ws HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    tls.flush().unwrap();

    let mut buf = [0u8; 64];
    let n = tls.read(&mut buf).expect("server should push while reading");

    assert!(
        String::from_utf8_lossy(&buf[..n]).contains("pushed"),
        "expected a pushed message, got {:?}",
        String::from_utf8_lossy(&buf[..n])
    );
    assert!(pushed.load(Ordering::SeqCst));
}

#[test]
fn plain_http_client_cannot_talk_to_the_tls_port() {
    if !have_openssl() {
        return;
    }
    let dir = scratch("plainvstls");
    let Some((cert_pem, key_pem, _)) = make_identity(&dir) else {
        return;
    };

    let addr = spawn_https(&cert_pem, &key_pem);

    // Speaking cleartext HTTP at a TLS listener must fail rather than be
    // answered, since the server sees a broken ClientHello.
    let mut sock = std::net::TcpStream::connect(addr).unwrap();
    sock.set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let _ = sock.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n");

    let mut buf = Vec::new();
    let _ = sock.read_to_end(&mut buf);

    assert!(
        !String::from_utf8_lossy(&buf).starts_with("HTTP/"),
        "a TLS port must not answer cleartext HTTP"
    );
}
