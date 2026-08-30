//! WebSocket framing (RFC 6455) on top of the HTTP server.
//!
//! This is a codec, not a connection manager: it turns a connection into
//! frames and back. Who owns the socket, and when it is read or written, is up
//! to the caller. That makes it usable both from a per-connection thread and
//! from a central game loop that broadcasts to many peers.
//!
//! The halves are [`ConnReader`] and [`ConnWriter`] rather than `TcpStream`,
//! so the same code serves `ws://` and `wss://`. A TLS session cannot be
//! cloned into independent halves the way a socket can, which is why the
//! writer is shared rather than duplicated.

use std::io::{self, BufReader, Read, Write};
use std::net::TcpStream;

use super::conn::{ConnReader, ConnWriter};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha1_smol::Sha1;

use super::{Request, Response, Status};

/// Magic GUID from RFC 6455, appended to the client key before hashing.
const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// Frames larger than this are rejected. A client claiming a 4 GB payload must
/// not be able to make the server allocate it.
const MAX_PAYLOAD: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    Continuation,
    Text,
    Binary,
    Close,
    Ping,
    Pong,
}

impl Opcode {
    fn from_bits(bits: u8) -> Option<Self> {
        Some(match bits {
            0x0 => Self::Continuation,
            0x1 => Self::Text,
            0x2 => Self::Binary,
            0x8 => Self::Close,
            0x9 => Self::Ping,
            0xA => Self::Pong,
            _ => return None,
        })
    }

    fn bits(self) -> u8 {
        match self {
            Self::Continuation => 0x0,
            Self::Text => 0x1,
            Self::Binary => 0x2,
            Self::Close => 0x8,
            Self::Ping => 0x9,
            Self::Pong => 0xA,
        }
    }

    pub fn is_control(self) -> bool {
        matches!(self, Self::Close | Self::Ping | Self::Pong)
    }
}

/// A fully reassembled application message.
#[derive(Debug)]
pub enum Message {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    /// Close with optional status code and reason.
    Close(Option<(u16, String)>),
}

#[derive(Debug)]
pub enum WsError {
    /// Peer violated the protocol; the connection must be closed.
    Protocol(&'static str),
    /// Payload exceeded `MAX_PAYLOAD`.
    TooLarge,
    Io(io::Error),
}

impl From<io::Error> for WsError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::fmt::Display for WsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Protocol(m) => write!(f, "websocket protocol error: {m}"),
            Self::TooLarge => write!(f, "websocket payload too large"),
            Self::Io(e) => write!(f, "websocket io error: {e}"),
        }
    }
}

/// Computes the `Sec-WebSocket-Accept` value for a client key.
pub fn accept_key(client_key: &str) -> String {
    let mut sha = Sha1::new();
    sha.update(client_key.as_bytes());
    sha.update(WS_GUID.as_bytes());
    STANDARD.encode(sha.digest().bytes())
}

/// Builds the 101 response that completes the handshake.
///
/// Returns `None` if the request is not a valid upgrade.
pub fn handshake_response(req: &Request) -> Option<Response> {
    let key = req.websocket_key()?;

    // The client must announce version 13; anything else we cannot speak.
    if req.headers.get("Sec-WebSocket-Version").map(str::trim) != Some("13") {
        return None;
    }

    Some(
        Response::new(Status::SwitchingProtocols)
            .with_header("Upgrade", "websocket")
            .with_header("Connection", "Upgrade")
            .with_header("Sec-WebSocket-Accept", accept_key(key)),
    )
}

/// Reads WebSocket frames from a stream and reassembles fragmented messages.
pub struct WsReader {
    reader: BufReader<ConnReader>,
    /// Payload accumulated across continuation frames.
    fragment: Vec<u8>,
    /// Opcode of the message being fragmented, if any.
    fragment_opcode: Option<Opcode>,
}

impl WsReader {
    pub fn new(stream: ConnReader) -> Self {
        Self::from_buffered(BufReader::with_capacity(16 * 1024, stream))
    }

    /// Continues from a reader that may already hold buffered bytes.
    ///
    /// Use this after a hijack: the HTTP server's reader can have frames in
    /// it that the client pipelined behind the upgrade request, and starting
    /// a fresh reader would silently drop them.
    pub fn from_buffered(reader: BufReader<ConnReader>) -> Self {
        Self {
            reader,
            fragment: Vec::new(),
            fragment_opcode: None,
        }
    }

    /// Blocks until a complete message arrives.
    ///
    /// Control frames are returned as-is; they may arrive interleaved between
    /// fragments of a data message and never disturb reassembly.
    pub fn read_message(&mut self) -> Result<Message, WsError> {
        loop {
            let frame = self.read_frame()?;

            if frame.opcode.is_control() {
                // Control frames must not be fragmented and are size-limited.
                if !frame.fin {
                    return Err(WsError::Protocol("fragmented control frame"));
                }
                if frame.payload.len() > 125 {
                    return Err(WsError::Protocol("oversized control frame"));
                }

                return Ok(match frame.opcode {
                    Opcode::Ping => Message::Ping(frame.payload),
                    Opcode::Pong => Message::Pong(frame.payload),
                    Opcode::Close => Message::Close(parse_close(&frame.payload)),
                    _ => unreachable!("checked by is_control"),
                });
            }

            match frame.opcode {
                Opcode::Continuation => {
                    let Some(opcode) = self.fragment_opcode else {
                        return Err(WsError::Protocol("continuation without start frame"));
                    };
                    if self.fragment.len() + frame.payload.len() > MAX_PAYLOAD {
                        return Err(WsError::TooLarge);
                    }
                    self.fragment.extend_from_slice(&frame.payload);

                    if frame.fin {
                        let payload = std::mem::take(&mut self.fragment);
                        self.fragment_opcode = None;
                        return finish(opcode, payload);
                    }
                }
                opcode @ (Opcode::Text | Opcode::Binary) => {
                    if self.fragment_opcode.is_some() {
                        return Err(WsError::Protocol("new message before previous finished"));
                    }
                    if frame.fin {
                        return finish(opcode, frame.payload);
                    }
                    self.fragment = frame.payload;
                    self.fragment_opcode = Some(opcode);
                }
                _ => unreachable!("control frames handled above"),
            }
        }
    }

    fn read_frame(&mut self) -> Result<Frame, WsError> {
        let mut head = [0u8; 2];
        self.reader.read_exact(&mut head)?;

        let fin = head[0] & 0b1000_0000 != 0;
        // Reserved bits must be zero unless an extension negotiated them.
        if head[0] & 0b0111_0000 != 0 {
            return Err(WsError::Protocol("reserved bits set"));
        }

        let opcode = Opcode::from_bits(head[0] & 0x0F)
            .ok_or(WsError::Protocol("unknown opcode"))?;

        let masked = head[1] & 0b1000_0000 != 0;
        // Every client-to-server frame must be masked.
        if !masked {
            return Err(WsError::Protocol("unmasked frame from client"));
        }

        let len = match head[1] & 0b0111_1111 {
            126 => {
                let mut b = [0u8; 2];
                self.reader.read_exact(&mut b)?;
                u16::from_be_bytes(b) as usize
            }
            127 => {
                let mut b = [0u8; 8];
                self.reader.read_exact(&mut b)?;
                let len = u64::from_be_bytes(b);
                // The high bit must be clear, and it must fit this platform.
                if len > MAX_PAYLOAD as u64 {
                    return Err(WsError::TooLarge);
                }
                len as usize
            }
            short => short as usize,
        };

        if len > MAX_PAYLOAD {
            return Err(WsError::TooLarge);
        }

        let mut mask = [0u8; 4];
        self.reader.read_exact(&mut mask)?;

        let mut payload = vec![0u8; len];
        self.reader.read_exact(&mut payload)?;
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }

        Ok(Frame {
            fin,
            opcode,
            payload,
        })
    }
}

struct Frame {
    fin: bool,
    opcode: Opcode,
    payload: Vec<u8>,
}

fn finish(opcode: Opcode, payload: Vec<u8>) -> Result<Message, WsError> {
    match opcode {
        Opcode::Binary => Ok(Message::Binary(payload)),
        Opcode::Text => String::from_utf8(payload)
            .map(Message::Text)
            .map_err(|_| WsError::Protocol("text frame is not valid UTF-8")),
        _ => Err(WsError::Protocol("unexpected opcode for data message")),
    }
}

fn parse_close(payload: &[u8]) -> Option<(u16, String)> {
    if payload.len() < 2 {
        return None;
    }
    let code = u16::from_be_bytes([payload[0], payload[1]]);
    let reason = String::from_utf8_lossy(&payload[2..]).into_owned();
    Some((code, reason))
}

/// Writes WebSocket frames to a stream.
///
/// Cloneable so a game loop can hold a writer per player while a separate
/// thread reads from the same socket.
pub struct WsWriter {
    stream: ConnWriter,
}

impl WsWriter {
    pub fn new(stream: ConnWriter) -> Self {
        Self { stream }
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            stream: self.stream.try_clone()?,
        })
    }

    /// The socket underneath, for timeouts and `set_nodelay`.
    pub fn socket(&self) -> io::Result<TcpStream> {
        self.stream.socket()
    }

    pub fn send_binary(&mut self, payload: &[u8]) -> io::Result<()> {
        self.send(Opcode::Binary, payload)
    }

    pub fn send_text(&mut self, text: &str) -> io::Result<()> {
        self.send(Opcode::Text, text.as_bytes())
    }

    pub fn send_ping(&mut self, payload: &[u8]) -> io::Result<()> {
        self.send(Opcode::Ping, payload)
    }

    pub fn send_pong(&mut self, payload: &[u8]) -> io::Result<()> {
        self.send(Opcode::Pong, payload)
    }

    pub fn send_close(&mut self, code: u16, reason: &str) -> io::Result<()> {
        let mut payload = Vec::with_capacity(2 + reason.len());
        payload.extend_from_slice(&code.to_be_bytes());
        payload.extend_from_slice(reason.as_bytes());
        self.send(Opcode::Close, &payload)
    }

    /// Server-to-client frames are never masked.
    fn send(&mut self, opcode: Opcode, payload: &[u8]) -> io::Result<()> {
        let mut frame = Vec::with_capacity(payload.len() + 10);
        frame.push(0b1000_0000 | opcode.bits());

        match payload.len() {
            n if n < 126 => frame.push(n as u8),
            n if n <= u16::MAX as usize => {
                frame.push(126);
                frame.extend_from_slice(&(n as u16).to_be_bytes());
            }
            n => {
                frame.push(127);
                frame.extend_from_slice(&(n as u64).to_be_bytes());
            }
        }

        frame.extend_from_slice(payload);
        self.stream.write_all(&frame)?;
        self.stream.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_rfc_example_accept_key() {
        // Example from RFC 6455 section 1.3.
        assert_eq!(
            accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }
}
