//! Protocol constants and the handshake structures this client uses.
//!
//! Only what a TLS 1.3 client needs is modelled. Values that exist purely
//! for backwards compatibility with TLS 1.2 middleboxes are written as fixed
//! bytes rather than given types, and are commented where they appear.

/// Content types for the record layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ContentType {
    ChangeCipherSpec = 20,
    Alert = 21,
    Handshake = 22,
    ApplicationData = 23,
}

impl ContentType {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            20 => Self::ChangeCipherSpec,
            21 => Self::Alert,
            22 => Self::Handshake,
            23 => Self::ApplicationData,
            _ => return None,
        })
    }
}

/// Handshake message types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HandshakeType {
    ClientHello = 1,
    ServerHello = 2,
    NewSessionTicket = 4,
    EncryptedExtensions = 8,
    Certificate = 11,
    CertificateRequest = 13,
    CertificateVerify = 15,
    Finished = 20,
    KeyUpdate = 24,
}

impl HandshakeType {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            1 => Self::ClientHello,
            2 => Self::ServerHello,
            4 => Self::NewSessionTicket,
            8 => Self::EncryptedExtensions,
            11 => Self::Certificate,
            13 => Self::CertificateRequest,
            15 => Self::CertificateVerify,
            20 => Self::Finished,
            24 => Self::KeyUpdate,
            _ => return None,
        })
    }
}

/// Cipher suites. Only TLS 1.3 AEAD suites, and only the two whose
/// primitives `ring` provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherSuite {
    Aes128GcmSha256,
    ChaCha20Poly1305Sha256,
}

impl CipherSuite {
    pub fn as_u16(self) -> u16 {
        match self {
            Self::Aes128GcmSha256 => 0x1301,
            Self::ChaCha20Poly1305Sha256 => 0x1303,
        }
    }

    pub fn from_u16(v: u16) -> Option<Self> {
        Some(match v {
            0x1301 => Self::Aes128GcmSha256,
            0x1303 => Self::ChaCha20Poly1305Sha256,
            _ => return None,
        })
    }

    pub fn key_len(self) -> usize {
        match self {
            Self::Aes128GcmSha256 => 16,
            Self::ChaCha20Poly1305Sha256 => 32,
        }
    }

    pub fn aead(self) -> &'static ring::aead::Algorithm {
        match self {
            Self::Aes128GcmSha256 => &ring::aead::AES_128_GCM,
            Self::ChaCha20Poly1305Sha256 => &ring::aead::CHACHA20_POLY1305,
        }
    }
}

/// Extension numbers used in our ClientHello.
pub mod ext {
    pub const SERVER_NAME: u16 = 0;
    pub const SUPPORTED_GROUPS: u16 = 10;
    pub const SIGNATURE_ALGORITHMS: u16 = 13;
    pub const ALPN: u16 = 16;
    pub const SUPPORTED_VERSIONS: u16 = 43;
    pub const KEY_SHARE: u16 = 51;
}

/// Named groups for key exchange. X25519 only: it is the one modern curve
/// `ring::agreement` exposes that needs no point validation of our own.
pub const GROUP_X25519: u16 = 0x001d;

/// Signature schemes we advertise and accept in CertificateVerify.
pub mod sig {
    pub const ECDSA_NISTP256_SHA256: u16 = 0x0403;
    pub const ECDSA_NISTP384_SHA384: u16 = 0x0503;
    pub const RSA_PSS_SHA256: u16 = 0x0804;
    pub const RSA_PSS_SHA384: u16 = 0x0805;
    pub const RSA_PSS_SHA512: u16 = 0x0806;
    pub const RSA_PKCS1_SHA256: u16 = 0x0401;
    pub const RSA_PKCS1_SHA384: u16 = 0x0501;
    pub const RSA_PKCS1_SHA512: u16 = 0x0601;

    /// Everything we are willing to verify, in preference order.
    pub const ALL: &[u16] = &[
        ECDSA_NISTP256_SHA256,
        ECDSA_NISTP384_SHA384,
        RSA_PSS_SHA256,
        RSA_PSS_SHA384,
        RSA_PSS_SHA512,
        RSA_PKCS1_SHA256,
        RSA_PKCS1_SHA384,
        RSA_PKCS1_SHA512,
    ];
}

pub const VERSION_TLS12: u16 = 0x0303;
pub const VERSION_TLS13: u16 = 0x0304;

/// Alert levels and the descriptions we send.
pub mod alert {
    pub const LEVEL_FATAL: u8 = 2;

    pub const CLOSE_NOTIFY: u8 = 0;
    pub const UNEXPECTED_MESSAGE: u8 = 10;
    pub const BAD_RECORD_MAC: u8 = 20;
    pub const HANDSHAKE_FAILURE: u8 = 40;
    pub const BAD_CERTIFICATE: u8 = 42;
    pub const CERTIFICATE_EXPIRED: u8 = 45;
    pub const CERTIFICATE_UNKNOWN: u8 = 46;
    pub const ILLEGAL_PARAMETER: u8 = 47;
    pub const UNKNOWN_CA: u8 = 48;
    pub const DECODE_ERROR: u8 = 50;
    pub const PROTOCOL_VERSION: u8 = 70;

    pub fn describe(code: u8) -> &'static str {
        match code {
            CLOSE_NOTIFY => "close_notify",
            UNEXPECTED_MESSAGE => "unexpected_message",
            BAD_RECORD_MAC => "bad_record_mac",
            HANDSHAKE_FAILURE => "handshake_failure",
            BAD_CERTIFICATE => "bad_certificate",
            CERTIFICATE_EXPIRED => "certificate_expired",
            CERTIFICATE_UNKNOWN => "certificate_unknown",
            ILLEGAL_PARAMETER => "illegal_parameter",
            UNKNOWN_CA => "unknown_ca",
            DECODE_ERROR => "decode_error",
            PROTOCOL_VERSION => "protocol_version",
            _ => "unknown alert",
        }
    }
}

/// The maximum plaintext a single record may carry.
pub const MAX_FRAGMENT: usize = 16384;

/// Context strings for CertificateVerify, per RFC 8446 §4.4.3.
pub const SERVER_CERT_VERIFY_CONTEXT: &[u8] = b"TLS 1.3, server CertificateVerify";
