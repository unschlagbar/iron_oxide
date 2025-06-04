use std::{collections::VecDeque, io::{Read, Write}, net::{SocketAddr, TcpStream}, sync::{Arc, RwLock}, time::Duration};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use sha1_smol::Sha1;

#[derive(Debug)]
pub struct WebSocket {
    stream: TcpStream,
    send_queue: VecDeque<Vec<u8>>,
    close: bool,
}

#[allow(dead_code)]
impl WebSocket {

    pub fn try_connect(mut stream: TcpStream, handshake_key: &str) -> Option<Self> {

        let mut sha = Sha1::new();
        sha.update(handshake_key.as_bytes());
        sha.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
        let umbrella = sha.digest().bytes();
        let mut encoded_key = [0; 28];
        STANDARD.encode_slice(umbrella, &mut encoded_key).unwrap();

        let response = format!("HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Accept: {}\r\n\r\n", unsafe {String::from_utf8_unchecked(encoded_key.to_vec())});
        stream.write(response.as_bytes()).unwrap();
        stream.flush().unwrap();
        stream.set_read_timeout(Some(Duration::from_millis(1000))).unwrap();

        let ws = Self { stream, send_queue: VecDeque::with_capacity(10), close: false };
        Some(ws)
    }

    pub fn new(stream: TcpStream) -> Self {
        let ws = WebSocket { stream, send_queue: VecDeque::with_capacity(10), close: false };
        ws
    }

    pub fn close(&mut self) {
        self.close = true;
    }

    pub fn send_ping(&mut self) {
        let mut ping = Vec::with_capacity(2);
        ping.push(0b10001001); // FIN und Opcode für Ping
        ping.push(1);
        ping.push(95);        // Länge 0 für Ping
        self.send_queue.push_back(ping);
    }

    pub fn send_pong(&mut self, data: Vec<u8>) {
        let mut pong = Vec::with_capacity(data.len() + 2);
        pong.push(0b10001000); // FIN und Opcode für Pong
        pong.push(data.len() as u8);  // Länge des Pongs
        pong.extend_from_slice(&data); // Daten des Pongs
        self.send_queue.push_back(pong);
    }

    pub fn send(&mut self, message: &[u8], msg_type: MessageDataType) {
        let mut m: Vec<u8> = Vec::with_capacity(message.len() + 14);
        let len = message.len();
        //let mask: [u8; 4] = [milis ^ 196, milis ^ 27, milis ^ 157, milis ^ 186];

        match len {
            0..=125 => {
                m.push(0b10000000 | msg_type as u8);
                m.push(len as u8);
                m.extend_from_slice(message);
            }
            0..=65535 => {
                m.push(0b10000000 | msg_type as u8);
                m.push(126);
                m.extend_from_slice(&(len as u16).to_be_bytes());
                m.extend_from_slice(message);
            },
            _ => {
                m.push(0b10000000 | msg_type as u8);
                m.push(127);
                m.extend_from_slice(&(len as u64).to_be_bytes());
                m.extend_from_slice(message);
            }
        }
        self.send_queue.push_back(m);
    }

    fn get_payloadlen(data: &[u8]) -> MessageLen {
        if data.len() < 3 {
            return MessageLen::ToShort;
        }
        let len  = data[1] & 0b01111111;

        let actual_len: u64 = match len {
            126 => {
                if data.len() < 4 {
                    return MessageLen::MissingBytes(4 - data.len());
                }
                let len = ((data[2] as u64) << 8) | (data[3] as u64);
                len + 8
            }
            127 => {
                if data.len() < 10 {
                    return MessageLen::MissingBytes(10 - data.len());
                }
                let len = ((data[2] as u64) << 56) | ((data[3] as u64) << 48) |
                ((data[4] as u64) << 40) | ((data[5] as u64) << 32) |
                ((data[6] as u64) << 24) | ((data[7] as u64) << 16) |
                ((data[8] as u64) << 8) | (data[9] as u64);
                len + 14
            }
            _ => len as u64 + 6
        };

        MessageLen::Len(actual_len as usize)
    }

    /// **Verarbeitet ausgehende Nachrichten**
    fn flush(&mut self) -> Option<()> {
        while let Some(message) = self.send_queue.pop_front() {
            if let Err(_) = self.stream.write_all(&message) {
                return None;
            }
        }
        self.stream.flush().unwrap();
        Some(())
    }

    pub fn run(ws_interface: Arc<RwLock<impl WebSocketInterface>>) {
        let mut stream;
        {
            let interface = ws_interface.read().unwrap();
            stream = interface.websocket().stream.try_clone().unwrap();
        }
    
        let ip = stream.peer_addr().unwrap();
        let mut buffer = [0; 8192];
        
        let mut message_buffer: Vec<u8> = Vec::new();
        let mut incomplete_message: Vec<u8> = Vec::new();
        let mut required_length: Option<usize> = None;
        let mut is_fragmented = false;
        let mut bytes_processed;
        let mut expected_fragment_type = None;
    
        loop {
            stream.take_error().expect("No error was expected...");

            //if  
    
            match stream.read(&mut buffer) {
                Ok(0) => {
                    println!("Connection closed");
                    let client = ws_interface.write().unwrap();
                    client.on_closed(ip);
                    return;
                },
                Ok(bytes_read) => {
                    bytes_processed = 0;
    
                    while bytes_processed < bytes_read {
                        if !incomplete_message.is_empty() {
                            let remaining_bytes = match required_length {
                                Some(len) => len.saturating_sub(incomplete_message.len()),
                                None => {
                                    // Wenn wir noch keine Länge haben, versuchen wir sie zu bestimmen
                                    match Self::get_payloadlen(&incomplete_message) {
                                        MessageLen::Len(total_len) => {
                                            // Sobald wir die Länge kennen, reservieren wir den Speicher
                                            incomplete_message.reserve(total_len);
                                            required_length = Some(total_len);
                                            total_len.saturating_sub(incomplete_message.len())
                                        },
                                        MessageLen::MissingBytes(missing) => missing,
                                        MessageLen::ToShort => {
                                            let to_add = std::cmp::min(
                                                bytes_read - bytes_processed,
                                                3 - incomplete_message.len()
                                            );
                                            incomplete_message.extend_from_slice(
                                                &buffer[bytes_processed..bytes_processed + to_add]
                                            );
                                            bytes_processed += to_add;
                                            continue;
                                        }
                                    }
                                }
                            };
    
                            let bytes_to_add = std::cmp::min(bytes_read - bytes_processed, remaining_bytes);
                            incomplete_message.extend_from_slice(
                                &buffer[bytes_processed..bytes_processed + bytes_to_add]
                            );
                            bytes_processed += bytes_to_add;
    
                            if let Some(len) = required_length {
                                if incomplete_message.len() >= len {
                                    Self::process_message(
                                        &incomplete_message,
                                        &mut is_fragmented,
                                        &mut message_buffer,
                                        &mut expected_fragment_type,
                                        ws_interface.clone(),
                                        &mut stream,
                                        ip
                                    );
                                    incomplete_message.clear();
                                    required_length = None;
                                }
                            }
                            continue;
                        }
    
                        match Self::get_payloadlen(&buffer[bytes_processed..bytes_read]) {
                            MessageLen::Len(len) => {
                                if bytes_processed + len <= bytes_read {
                                    Self::process_message(
                                        &buffer[bytes_processed..bytes_processed + len],
                                        &mut is_fragmented,
                                        &mut message_buffer,
                                        &mut expected_fragment_type,
                                        ws_interface.clone(),
                                        &mut stream,
                                        ip
                                    );
                                    bytes_processed += len;
                                } else {
                                    // Reserviere die Kapazität für die komplette Nachricht
                                    incomplete_message = Vec::with_capacity(len);
                                    incomplete_message.extend_from_slice(
                                        &buffer[bytes_processed..bytes_read]
                                    );
                                    required_length = Some(len);
                                    bytes_processed = bytes_read;
                                }
                            },
                            MessageLen::MissingBytes(missing) => {
                                // Reserviere initial nur für den Header
                                incomplete_message = Vec::with_capacity(missing + 10);
                                incomplete_message.extend_from_slice(
                                    &buffer[bytes_processed..bytes_read]
                                );
                                required_length = Some(missing);
                                bytes_processed = bytes_read;
                            },
                            MessageLen::ToShort => {
                                // Reserviere initial für einen kleinen Header
                                incomplete_message = Vec::with_capacity(14);
                                incomplete_message.extend_from_slice(
                                    &buffer[bytes_processed..bytes_read]
                                );
                                bytes_processed = bytes_read;
                            }
                        }
                    }
                },
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock => (),
                Err(e) => {
                    println!("Error occurred: {e}");
                    let client = ws_interface.write().unwrap();
                    client.on_closed(ip);
                    return;
                }
            }
    
            let mut client = ws_interface.write().unwrap();
            let ws = client.websocket_mut();
            if ws.flush().is_none() {
                client.on_closed(ip);
                return;
            } else if ws.close {
                return;
            }
        }
    }

    fn process_message(
        message: &[u8],
        is_frag: &mut bool,
        message_buffer: &mut Vec<u8>,
        expected_type: &mut Option<u8>,
        ws_interface: Arc<RwLock<impl WebSocketInterface>>,
        stream: &mut TcpStream,
        ip: SocketAddr
    ) {
        let fin = (message[0] & 0b10000000) != 0;
        let opt_code = message[0] & 0b00001111;
        let len = message[1] & 0b01111111;
        let mask_bit = (message[1] & 0b10000000) != 0;
    
        if !mask_bit {
            return;
        }
    
        let mut offset = 2;
        
        let actual_len: u64 = match len {
            126 => {
                let len = ((message[offset] as u64) << 8) | (message[offset + 1] as u64);
                offset += 2;
                len
            }
            127 => {
                let len = ((message[offset] as u64) << 56) | ((message[offset + 1] as u64) << 48) |
                ((message[offset + 2] as u64) << 40) | ((message[offset + 3] as u64) << 32) |
                ((message[offset + 4] as u64) << 24) | ((message[offset + 5] as u64) << 16) |
                ((message[offset + 6] as u64) << 8) | (message[offset + 7] as u64);
                offset += 8;
                len
            }
            _ => len as u64
        };
    
        let mask = [
            message[offset],
            message[offset + 1],
            message[offset + 2],
            message[offset + 3]
        ];
        offset += 4;
    
        let mut data = Vec::with_capacity(actual_len as usize);
        data.extend_from_slice(&message[offset..]);
    
        for i in 0..data.len() {
            data[i] ^= mask[i % 4];
        }
    
        //println!("Processing message: fin={}, opcode={}, is_frag={}", fin, opt_code, is_frag);
    
        match opt_code {
            0 => {
                // Continuation Frame
                if *is_frag {
                    message_buffer.extend_from_slice(&data);
                    if fin {
                        let mut client = ws_interface.write().unwrap();
                        client.on_message(message_buffer.clone());
                        message_buffer.clear();
                        *is_frag = false;
                        *expected_type = None;
                        //println!("Fragmented message completed");
                    }
                } else {
                    println!("Received continuation frame without starting frame");
                }
            }
            1 | 2 => {
                // Text or Binary Frame
                if *is_frag {
                    println!("Received new message while still processing fragments");
                    return;
                }
                
                if fin {
                    // Complete message in single frame
                    let mut client = ws_interface.write().unwrap();
                    client.on_message(data);
                    //println!("Complete message received");
                } else {
                    // Start of fragmented message
                    message_buffer.clear();
                    message_buffer.extend_from_slice(&data);
                    *is_frag = true;
                    *expected_type = Some(opt_code);
                    //println!("Started fragmented message");
                }
            }
            8 => {
                // Close Frame
                let mut close_frame = Vec::with_capacity(data.len() + 2);
                close_frame.push(0b10001000);
                close_frame.push(data.len() as u8);
                close_frame.extend_from_slice(&data);
                let _ = stream.write_all(&close_frame);
                let client = ws_interface.write().unwrap();
                client.on_closed(ip);
            }
            9 => {
                // Ping Frame
                let mut pong = Vec::with_capacity(data.len() + 2);
                pong.push(0b10001010);
                pong.push(data.len() as u8);
                pong.extend_from_slice(&data);
                let _ = stream.write_all(&pong);
            }
            _ => println!("Unhandled opcode: {}", opt_code),
        }
    }


    pub fn ip(&self) -> SocketAddr {
        self.stream.peer_addr().unwrap()
    }
}

impl Clone for WebSocket {
    fn clone(&self) -> Self {
        Self { stream: self.stream.try_clone().unwrap(), send_queue: VecDeque::with_capacity(10), close: false }
    }
}

enum MessageLen {
    Len(usize),
    MissingBytes(usize),
    ToShort,
}

pub enum MessageDataType {
    Continue,
    Text,
    Binary,
}

pub trait WebSocketInterface {
    fn on_message(&mut self, data: Vec<u8>);
    fn on_closed(&self, ip: SocketAddr);
    fn websocket(&self) -> &WebSocket;
    fn websocket_mut(&mut self) -> &mut WebSocket;
}