//! Chain validation against `openssl s_server`, with test CAs made by `openssl`.
//!
//! The whole file is skipped when `openssl` is not on PATH.

#![cfg(feature = "tls")]

use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use iron_oxide::net::http::tls::{TlsConfig, TlsError, TlsStream, Verifier};

const CA: &str = "basicConstraints=critical,CA:TRUE\nkeyUsage=critical,keyCertSign,cRLSign\n";
const INTER: &str = "basicConstraints=critical,CA:TRUE,pathlen:0\nkeyUsage=critical,keyCertSign,cRLSign\n";
const LEAF: &str = "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\n\
                    extendedKeyUsage=serverAuth\nsubjectAltName=DNS:localhost\n";

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
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

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("iron_oxide_chain_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn openssl(dir: &Path, args: &[&str]) {
    let status = Command::new("openssl")
        .current_dir(dir)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "openssl {args:?} failed");
}

/// Issues `name.pem` and `name.key`, self-signed when `issuer` is `None`.
fn issue(dir: &Path, name: &str, issuer: Option<&str>, ext: &str, extra: &[&str]) {
    let (key, csr, pem, extfile) = (
        format!("{name}.key"),
        format!("{name}.csr"),
        format!("{name}.pem"),
        format!("{name}.ext"),
    );
    std::fs::write(dir.join(&extfile), ext).unwrap();
    openssl(dir, &["genpkey", "-algorithm", "EC", "-pkeyopt", "ec_paramgen_curve:P-256", "-out", &key]);
    openssl(dir, &["req", "-new", "-key", &key, "-subj", &format!("/CN={name}"), "-out", &csr]);

    let mut args = vec!["x509", "-req", "-in", &csr, "-extfile", &extfile, "-out", &pem];
    let (ca, ca_key);
    match issuer {
        Some(issuer) => {
            ca = format!("{issuer}.pem");
            ca_key = format!("{issuer}.key");
            args.extend(["-CA", &ca, "-CAkey", &ca_key, "-CAcreateserial"]);
        }
        None => args.extend(["-key", &key]),
    }
    if extra.is_empty() {
        args.extend(["-days", "30"]);
    }
    args.extend(extra);
    openssl(dir, &args);
}

fn der(dir: &Path, name: &str) -> Vec<u8> {
    let out = format!("{name}.der");
    openssl(dir, &["x509", "-in", &format!("{name}.pem"), "-outform", "DER", "-out", &out]);
    std::fs::read(dir.join(out)).unwrap()
}

/// Serves `leaf` with the intermediates in `chain`, leaf-side first.
fn serve(dir: &Path, leaf: &str, chain: &[&str], port: u16) -> Server {
    let bundle = dir.join("chain.pem");
    let pems: Vec<u8> = chain
        .iter()
        .flat_map(|name| std::fs::read(dir.join(format!("{name}.pem"))).unwrap())
        .collect();
    std::fs::write(&bundle, pems).unwrap();

    let mut cmd = Command::new("openssl");
    cmd.current_dir(dir)
        .args(["s_server", "-accept", &port.to_string()])
        .args(["-cert", &format!("{leaf}.pem"), "-key", &format!("{leaf}.key")])
        .args(["-tls1_3", "-www", "-quiet"]);
    if !chain.is_empty() {
        cmd.arg("-cert_chain").arg(&bundle);
    }
    let child = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .unwrap();

    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Server(child)
}

fn connect(port: u16, hostname: &str, roots: Vec<Vec<u8>>) -> Result<(), TlsError> {
    let tcp = TcpStream::connect(("127.0.0.1", port)).unwrap();
    tcp.set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .unwrap();
    let config = TlsConfig {
        verifier: Verifier::roots(&roots)?,
        alpn: Vec::new(),
    };
    TlsStream::connect(tcp, hostname, &config).map(|_| ())
}

fn assert_rejected(result: Result<(), TlsError>) {
    let err = result.err().expect("the chain must be rejected");
    assert!(matches!(err, TlsError::BadCertificate(_)), "got {err:?}");
}

/// root -> inter -> leaf, the shape of a public CA.
fn standard(name: &str) -> PathBuf {
    let dir = scratch(name);
    issue(&dir, "root", None, CA, &[]);
    issue(&dir, "inter", Some("root"), INTER, &[]);
    issue(&dir, "leaf", Some("inter"), LEAF, &[]);
    dir
}

#[test]
fn accepts_a_chain_to_a_configured_root() {
    if !have_openssl() {
        return;
    }
    let dir = standard("ok");
    let _server = serve(&dir, "leaf", &["inter"], 14510);
    connect(14510, "localhost", vec![der(&dir, "root")]).expect("the chain is valid");
}

#[test]
fn rejects_a_chain_to_another_root() {
    if !have_openssl() {
        return;
    }
    let dir = standard("otherroot");
    issue(&dir, "other", None, CA, &[]);
    let _server = serve(&dir, "leaf", &["inter"], 14511);
    assert_rejected(connect(14511, "localhost", vec![der(&dir, "other")]));
}

#[test]
fn rejects_a_hostname_the_leaf_does_not_cover() {
    if !have_openssl() {
        return;
    }
    let dir = standard("host");
    let _server = serve(&dir, "leaf", &["inter"], 14512);
    assert_rejected(connect(14512, "example.com", vec![der(&dir, "root")]));
}

#[test]
fn rejects_a_missing_intermediate() {
    if !have_openssl() {
        return;
    }
    let dir = standard("missing");
    let _server = serve(&dir, "leaf", &[], 14513);
    assert_rejected(connect(14513, "localhost", vec![der(&dir, "root")]));
}

#[test]
fn rejects_an_intermediate_that_is_not_a_ca() {
    if !have_openssl() {
        return;
    }
    let dir = scratch("notca");
    issue(&dir, "root", None, CA, &[]);
    issue(&dir, "inter", Some("root"), "basicConstraints=critical,CA:FALSE\n", &[]);
    issue(&dir, "leaf", Some("inter"), LEAF, &[]);
    let _server = serve(&dir, "leaf", &["inter"], 14514);
    assert_rejected(connect(14514, "localhost", vec![der(&dir, "root")]));
}

#[test]
fn rejects_a_path_longer_than_path_len_allows() {
    if !have_openssl() {
        return;
    }
    let dir = standard("pathlen");
    issue(&dir, "inter2", Some("inter"), CA, &[]);
    issue(&dir, "leaf2", Some("inter2"), LEAF, &[]);
    let _server = serve(&dir, "leaf2", &["inter2", "inter"], 14515);
    assert_rejected(connect(14515, "localhost", vec![der(&dir, "root")]));
}

#[test]
fn rejects_an_expired_intermediate() {
    if !have_openssl() {
        return;
    }
    let dir = scratch("expired");
    issue(&dir, "root", None, CA, &[]);
    let old = ["-not_before", "20200101000000Z", "-not_after", "20210101000000Z"];
    issue(&dir, "inter", Some("root"), CA, &old);
    issue(&dir, "leaf", Some("inter"), LEAF, &[]);
    let _server = serve(&dir, "leaf", &["inter"], 14516);
    assert_rejected(connect(14516, "localhost", vec![der(&dir, "root")]));
}

#[test]
fn rejects_a_leaf_that_is_a_ca() {
    if !have_openssl() {
        return;
    }
    let dir = standard("leafca");
    issue(&dir, "caleaf", Some("inter"), &LEAF.replace("CA:FALSE", "CA:TRUE"), &[]);
    let _server = serve(&dir, "caleaf", &["inter"], 14517);
    assert_rejected(connect(14517, "localhost", vec![der(&dir, "root")]));
}
