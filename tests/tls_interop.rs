//! Interop tests against `openssl s_server`.
//!
//! These exist because a TLS implementation that only ever talks to itself
//! proves nothing: a bug in the key schedule or the transcript is perfectly
//! self-consistent until a real peer disagrees. Everything here therefore
//! runs against OpenSSL's TLS 1.3 server.
//!
//! The whole file is skipped when `openssl` is not on PATH.

#![cfg(feature = "tls")]

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};

use iron_oxide::net::http::tls::{TlsConfig, TlsStream, Verifier};

/// A self-signed certificate plus the server holding it.
struct TestServer {
    child: Child,
    port: u16,
    cert_der: Vec<u8>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn have_openssl() -> bool {
    Command::new("openssl")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Generates a self-signed cert for `cn` and starts a server on a free port.
fn start_server(dir: &std::path::Path, cn: &str, port: u16) -> Option<TestServer> {
    let cert = dir.join(format!("{cn}-cert.pem"));
    let key = dir.join(format!("{cn}-key.pem"));
    let der = dir.join(format!("{cn}-cert.der"));

    let ok = Command::new("openssl")
        .args([
            "req", "-x509", "-newkey", "rsa:2048", "-keyout",
        ])
        .arg(&key)
        .arg("-out")
        .arg(&cert)
        .args(["-days", "3650", "-nodes", "-subj"])
        .arg(format!("/CN={cn}"))
        .arg("-addext")
        .arg(format!("subjectAltName=DNS:{cn}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    if !ok.success() {
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

    let child = Command::new("openssl")
        .args(["s_server", "-accept", &port.to_string(), "-cert"])
        .arg(&cert)
        .arg("-key")
        .arg(&key)
        .args(["-tls1_3", "-www", "-quiet"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .ok()?;

    // Wait for the listener rather than sleeping a fixed amount.
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Some(TestServer {
        child,
        port,
        cert_der: std::fs::read(&der).ok()?,
    })
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("iron_oxide_tls_{name}"));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn connect(
    server: &TestServer,
    hostname: &str,
    verifier: Verifier,
) -> Result<TlsStream<TcpStream>, iron_oxide::net::http::tls::TlsError> {
    let tcp = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    tcp.set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .unwrap();
    let config = TlsConfig {
        verifier,
        alpn: Vec::new(),
    };
    TlsStream::connect(tcp, hostname, &config)
}

#[test]
fn handshake_and_exchange_data_with_openssl() {
    if !have_openssl() {
        eprintln!("skipping: openssl not available");
        return;
    }
    let dir = scratch("roundtrip");
    let Some(server) = start_server(&dir, "localhost", 14501) else {
        eprintln!("skipping: could not start openssl s_server");
        return;
    };

    let mut tls = connect(
        &server,
        "localhost",
        Verifier::Pinned(vec![server.cert_der.clone()]),
    )
    .expect("handshake should succeed against a pinned certificate");

    tls.write_all(b"GET / HTTP/1.0\r\n\r\n").unwrap();
    tls.flush().unwrap();

    let mut buf = Vec::new();
    let _ = tls.read_to_end(&mut buf);
    let text = String::from_utf8_lossy(&buf);

    assert!(
        text.starts_with("HTTP/1.0 200"),
        "expected a response, got {:?}",
        &text[..text.len().min(200)]
    );
}

#[test]
fn rejects_a_certificate_that_does_not_match_the_pin() {
    if !have_openssl() {
        return;
    }
    let dir = scratch("wrongpin");
    let Some(server) = start_server(&dir, "localhost", 14502) else {
        return;
    };
    let Some(other) = start_server(&dir, "evil.example", 14503) else {
        return;
    };

    // The server is genuine, but we pin a different certificate: this is the
    // shape of a man-in-the-middle, and it must not connect.
    let err = connect(
        &server,
        "localhost",
        Verifier::Pinned(vec![other.cert_der.clone()]),
    )
        .err()
    .expect("a mismatched pin must fail");

    assert!(
        matches!(
            err,
            iron_oxide::net::http::tls::TlsError::BadCertificate(_)
        ),
        "got {err:?}"
    );
}

#[test]
fn rejects_a_hostname_the_certificate_does_not_cover() {
    if !have_openssl() {
        return;
    }
    let dir = scratch("wronghost");
    let Some(server) = start_server(&dir, "evil.example", 14504) else {
        return;
    };

    // Pin matches, but we asked for a different name than the certificate
    // covers. Pinning does not excuse a hostname mismatch.
    let err = connect(
        &server,
        "localhost",
        Verifier::Pinned(vec![server.cert_der.clone()]),
    )
        .err()
    .expect("a hostname mismatch must fail");

    assert!(
        matches!(
            err,
            iron_oxide::net::http::tls::TlsError::BadCertificate(_)
        ),
        "got {err:?}"
    );
}

#[test]
fn empty_pin_set_refuses_everything() {
    if !have_openssl() {
        return;
    }
    let dir = scratch("nopins");
    let Some(server) = start_server(&dir, "localhost", 14505) else {
        return;
    };

    // The default configuration trusts nothing, so even a valid server is
    // refused until the caller says what to trust.
    let err = connect(&server, "localhost", Verifier::Pinned(Vec::new()))
        .err()
        .expect("an empty pin set must fail");

    assert!(
        matches!(
            err,
            iron_oxide::net::http::tls::TlsError::BadCertificate(_)
        ),
        "got {err:?}"
    );
}

#[test]
fn public_key_pinning_accepts_the_same_key() {
    if !have_openssl() {
        return;
    }
    let dir = scratch("keypin");
    let Some(server) = start_server(&dir, "localhost", 14506) else {
        return;
    };

    let leaf = iron_oxide::net::http::tls::Certificate::parse(&server.cert_der).unwrap();
    let mut tls = connect(
        &server,
        "localhost",
        Verifier::PinnedPublicKey(vec![leaf.public_key.clone()]),
    )
    .expect("pinning the public key should succeed");

    tls.write_all(b"GET / HTTP/1.0\r\n\r\n").unwrap();
    tls.flush().unwrap();
    let mut buf = Vec::new();
    let _ = tls.read_to_end(&mut buf);
    assert!(String::from_utf8_lossy(&buf).starts_with("HTTP/1.0 200"));
}
