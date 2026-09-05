//! Reading and writing the system clipboard.
//!
//! Wayland has no clipboard call: the selection is an object on a socket, so
//! this speaks the protocol itself. It opens its own connection rather than
//! borrowing the window's, which rules out the core `wl_data_device` — the
//! compositor offers that selection only to the client holding keyboard focus,
//! and a second connection has no surface and no focus. What is left is the
//! data-control protocol, the one clipboard managers use for exactly this
//! reason: `ext_data_control_v1`, or `zwlr_data_control_unstable_v1` on a
//! compositor that predates it.
//!
//! The two directions cost very different things. A paste is one connection,
//! used once and dropped — about a millisecond, no state kept, cheap enough to
//! do on the ui thread. A copy is not a call at all: the compositor takes a
//! promise rather than the bytes, and every paste by anyone comes back as an
//! event asking for them, so owning the selection means holding a connection
//! open for as long as the selection is ours. That runs on a thread of its own.

#[cfg(not(target_os = "linux"))]
pub fn text() -> Option<String> {
    None
}

#[cfg(not(target_os = "linux"))]
pub fn set_text(_text: String) {}

#[cfg(target_os = "linux")]
pub use linux::{set_text, text};

#[cfg(target_os = "linux")]
mod linux {
    use std::collections::VecDeque;
    use std::env;
    use std::fs::File;
    use std::io::{ErrorKind, Read, Write};
    use std::os::fd::{AsRawFd, FromRawFd, RawFd};
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;

    /// The text on the clipboard, or nothing if it is empty, holds something
    /// that is not text, or there is no compositor to ask.
    pub fn text() -> Option<String> {
        let mut conn = Conn::connect()?;
        conn.selection_text()
    }

    /// Puts `text` on the clipboard and keeps it there.
    ///
    /// Returns immediately; the connection that owns the selection outlives the
    /// call on a thread of its own, because the bytes are only ever asked for
    /// later. It ends when the compositor says the selection has been taken
    /// over, so copying twice costs one thread and not two — the second copy is
    /// what cancels the first.
    pub fn set_text(text: String) {
        std::thread::spawn(move || {
            if let Some(mut conn) = Conn::connect() {
                conn.own_selection(&text);
            }
        });
    }

    /// How long to wait for the owner of the selection to write it. It is
    /// another application on the other end of the pipe, so this is a guard
    /// against one that is wedged, not a normal cost.
    const READ_TIMEOUT_MS: i32 = 500;

    /// What the clipboard is asked for, best first. The first two are what
    /// anything modern offers; the rest are what X11 clients bring with them
    /// through the compositor's own bridge.
    const WANTED: [&str; 5] = [
        "text/plain;charset=utf-8",
        "UTF8_STRING",
        "text/plain",
        "STRING",
        "TEXT",
    ];

    /// The one object every connection starts with.
    const DISPLAY: u32 = 1;

    const DISPLAY_SYNC: u16 = 0;
    const DISPLAY_GET_REGISTRY: u16 = 1;
    const DISPLAY_ERROR: u16 = 0;

    const REGISTRY_BIND: u16 = 0;
    const REGISTRY_GLOBAL: u16 = 0;

    const CALLBACK_DONE: u16 = 0;

    /// Both data-control protocols lay their requests and events out the same
    /// way, so only the interface name differs between them.
    const MANAGER_CREATE_SOURCE: u16 = 0;
    const MANAGER_GET_DEVICE: u16 = 1;
    const DEVICE_SET_SELECTION: u16 = 0;
    const DEVICE_DATA_OFFER: u16 = 0;
    const DEVICE_SELECTION: u16 = 1;
    const OFFER_RECEIVE: u16 = 0;
    const OFFER_EVENT: u16 = 0;
    const SOURCE_OFFER: u16 = 0;
    const SOURCE_SEND: u16 = 0;
    const SOURCE_CANCELLED: u16 = 1;

    /// Newest first: a compositor offering both should be talked to in the
    /// language that is not deprecated.
    const MANAGERS: [&str; 2] = [
        "ext_data_control_manager_v1",
        "zwlr_data_control_manager_v1",
    ];

    struct Conn {
        socket: UnixStream,
        /// Bytes read from the socket that are not yet a whole message.
        buf: Vec<u8>,
        /// Descriptors that arrived beside the bytes, in the order the messages
        /// carrying them did — which is how a `send` event finds its pipe.
        fds: VecDeque<RawFd>,
        next_id: u32,
    }

    /// One event, as it came off the wire: which object sent it, which of that
    /// interface's events it is, and the arguments still packed.
    struct Event {
        object: u32,
        opcode: u16,
        args: Vec<u8>,
    }

    impl Conn {
        fn connect() -> Option<Self> {
            let display = env::var_os("WAYLAND_DISPLAY")?;
            let mut path = PathBuf::from(display);
            if path.is_relative() {
                let dir = env::var_os("XDG_RUNTIME_DIR")?;
                path = PathBuf::from(dir).join(path);
            }

            Some(Self {
                socket: UnixStream::connect(path).ok()?,
                buf: Vec::with_capacity(4096),
                fds: VecDeque::new(),
                // 1 is the display itself.
                next_id: 2,
            })
        }

        fn new_id(&mut self) -> u32 {
            let id = self.next_id;
            self.next_id += 1;
            id
        }

        fn send(&mut self, object: u32, opcode: u16, args: &[u8]) -> Option<()> {
            self.socket.write_all(&message(object, opcode, args)).ok()
        }

        /// Reads one event, blocking until a whole one has arrived.
        fn event(&mut self) -> Option<Event> {
            loop {
                if self.buf.len() >= 8 {
                    let size = u16::from_ne_bytes([self.buf[6], self.buf[7]]) as usize;
                    if size < 8 {
                        return None;
                    }
                    if self.buf.len() >= size {
                        let object = u32::from_ne_bytes(self.buf[0..4].try_into().unwrap());
                        let opcode = u16::from_ne_bytes([self.buf[4], self.buf[5]]);
                        let args = self.buf[8..size].to_vec();
                        self.buf.drain(..size);
                        return Some(Event {
                            object,
                            opcode,
                            args,
                        });
                    }
                }

                let mut chunk = [0; 4096];
                match self.recv(&mut chunk) {
                    Ok(0) => return None,
                    Ok(n) => self.buf.extend_from_slice(&chunk[..n]),
                    Err(e) if e.kind() == ErrorKind::Interrupted => (),
                    Err(_) => return None,
                }
            }
        }

        /// One read, keeping any descriptors that came with it. A `send` event
        /// carries the pipe to answer it on *beside* the bytes rather than in
        /// them, so a plain read would take the message and drop the pipe.
        fn recv(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let iov = IoVec {
                base: buf.as_mut_ptr(),
                len: buf.len(),
            };
            // Room for a handful of descriptors, aligned like the header that
            // precedes each of them.
            let mut control = [0u64; 16];
            let mut msg = MsgHdr {
                name: std::ptr::null_mut(),
                namelen: 0,
                iov: &iov,
                iovlen: 1,
                control: control.as_mut_ptr().cast(),
                controllen: size_of_val(&control),
                flags: 0,
            };

            let read = unsafe { recvmsg(self.socket.as_raw_fd(), &mut msg, 0) };
            if read < 0 {
                return Err(std::io::Error::last_os_error());
            }

            let mut at = 0;
            while at + size_of::<CmsgHdr>() <= msg.controllen {
                let header = unsafe { &*control.as_ptr().cast::<u8>().add(at).cast::<CmsgHdr>() };
                if header.len < size_of::<CmsgHdr>() {
                    break;
                }
                if header.level == SOL_SOCKET && header.kind == SCM_RIGHTS {
                    let count = (header.len - size_of::<CmsgHdr>()) / size_of::<RawFd>();
                    let first = unsafe {
                        control
                            .as_ptr()
                            .cast::<u8>()
                            .add(at + size_of::<CmsgHdr>())
                            .cast::<RawFd>()
                    };
                    for i in 0..count {
                        self.fds.push_back(unsafe { first.add(i).read_unaligned() });
                    }
                }
                at += header.len.next_multiple_of(size_of::<usize>());
            }

            Ok(read as usize)
        }

        /// Asks the compositor to answer once it has dealt with everything sent
        /// so far, and returns the id that answer will come back on. Every
        /// "have the events I expect arrived yet" question is really this one.
        fn sync(&mut self) -> Option<u32> {
            let callback = self.new_id();
            self.send(DISPLAY, DISPLAY_SYNC, &new_id(callback))?;
            Some(callback)
        }

        /// The registry, and the two globals both directions need out of it:
        /// a seat, and the newest data-control manager the compositor has.
        fn globals(&mut self) -> Option<(u32, u32, u32, usize)> {
            let registry = self.new_id();
            self.send(DISPLAY, DISPLAY_GET_REGISTRY, &new_id(registry))?;
            let fence = self.sync()?;

            // The compositor announces every global it has, then answers the
            // sync — so one pass gets the whole list.
            let mut seat = None;
            let mut manager = None;
            loop {
                let event = self.event()?;
                if event.object == fence && event.opcode == CALLBACK_DONE {
                    break;
                }
                if event.object == DISPLAY && event.opcode == DISPLAY_ERROR {
                    return None;
                }
                if event.object != registry || event.opcode != REGISTRY_GLOBAL {
                    continue;
                }

                let mut args = Args::new(&event.args);
                let name = args.uint()?;
                let interface = args.string()?;
                let version = args.uint()?;

                if interface == "wl_seat" {
                    seat = seat.or(Some((name, version)));
                } else if let Some(rank) = MANAGERS.iter().position(|m| *m == interface) {
                    // A lower rank is the newer protocol.
                    if manager.is_none_or(|(_, best)| rank < best) {
                        manager = Some((name, rank));
                    }
                }
            }

            let (seat_name, _) = seat?;
            let (manager_name, manager_rank) = manager?;
            Some((registry, seat_name, manager_name, manager_rank))
        }

        /// Binds one global by name, and hands back the id it now lives at.
        fn bind(&mut self, registry: u32, name: u32, interface: &str) -> Option<u32> {
            let id = self.new_id();
            self.send(
                registry,
                REGISTRY_BIND,
                &[uint(name), string(interface), uint(1), new_id(id)].concat(),
            )?;
            Some(id)
        }

        /// Takes the selection and answers every paste of it until someone else
        /// takes it back. The compositor never asks for the text up front, so
        /// this cannot be a request that returns: what it owns is the promise,
        /// and each `send` event is one pipe to keep it on.
        fn own_selection(&mut self, text: &str) -> Option<()> {
            let (registry, seat_name, manager_name, rank) = self.globals()?;
            let seat = self.bind(registry, seat_name, "wl_seat")?;
            let manager = self.bind(registry, manager_name, MANAGERS[rank])?;

            // The types are advertised before the source is handed over: after
            // `set_selection` the compositor has already told everyone what is
            // on offer, and a later `offer` would not reach them.
            let source = self.new_id();
            self.send(manager, MANAGER_CREATE_SOURCE, &new_id(source))?;
            for mime in WANTED {
                self.send(source, SOURCE_OFFER, &string(mime))?;
            }

            let device = self.new_id();
            self.send(
                manager,
                MANAGER_GET_DEVICE,
                &[new_id(device), object(seat)].concat(),
            )?;
            self.send(device, DEVICE_SET_SELECTION, &object(source))?;

            loop {
                let event = self.event()?;
                if event.object == DISPLAY && event.opcode == DISPLAY_ERROR {
                    return None;
                }
                if event.object != source {
                    continue;
                }
                match event.opcode {
                    // We offered nothing but text, so which type was asked for
                    // does not change what is written.
                    SOURCE_SEND => {
                        let fd = self.fds.pop_front()?;
                        let mut pipe = unsafe { File::from_raw_fd(fd) };
                        // A reader that gives up mid-paste is its own business:
                        // the selection is still ours and the next paste still
                        // has to be answered.
                        let _ = pipe.write_all(text.as_bytes());
                    }
                    // Someone else copied something. The selection is theirs,
                    // and this connection has nothing left to do.
                    SOURCE_CANCELLED => return Some(()),
                    _ => (),
                }
            }
        }

        fn selection_text(&mut self) -> Option<String> {
            let (registry, seat_name, manager_name, rank) = self.globals()?;
            let seat = self.bind(registry, seat_name, "wl_seat")?;
            let manager = self.bind(registry, manager_name, MANAGERS[rank])?;

            let device = self.new_id();
            self.send(
                manager,
                MANAGER_GET_DEVICE,
                &[new_id(device), object(seat)].concat(),
            )?;
            let fence = self.sync()?;

            // The device reports what is on the clipboard the moment it exists:
            // an offer, the types that offer can be read as, and finally which
            // offer is the selection.
            let mut offers: Vec<(u32, Vec<String>)> = Vec::new();
            let mut selection = None;
            loop {
                let event = self.event()?;
                if event.object == fence && event.opcode == CALLBACK_DONE {
                    break;
                }
                if event.object == DISPLAY && event.opcode == DISPLAY_ERROR {
                    return None;
                }

                let mut args = Args::new(&event.args);
                if event.object == device {
                    match event.opcode {
                        DEVICE_DATA_OFFER => offers.push((args.uint()?, Vec::new())),
                        DEVICE_SELECTION => selection = Some(args.uint()?),
                        _ => (),
                    }
                } else if event.opcode == OFFER_EVENT
                    && let Some(offer) = offers.iter_mut().find(|(id, _)| *id == event.object)
                    && let Some(mime) = args.string()
                {
                    offer.1.push(mime.to_string());
                }
            }

            // Nothing on the clipboard at all is a zero id, not a missing event.
            let selection = selection.filter(|id| *id != 0)?;
            let mimes = &offers.iter().find(|(id, _)| *id == selection)?.1;
            let mime = WANTED
                .iter()
                .find(|want| mimes.iter().any(|have| have == *want))?;

            self.read_offer(selection, mime)
        }

        /// Hands the offer's owner a pipe to write the selection into, and reads
        /// the other end.
        fn read_offer(&mut self, offer: u32, mime: &str) -> Option<String> {
            let (mut reader, writer) = std::io::pipe().ok()?;

            let args = string(mime);
            send_with_fd(
                self.socket.as_raw_fd(),
                &message(offer, OFFER_RECEIVE, &args),
                writer.as_raw_fd(),
            )?;

            // The writing end has to be ours alone from here: the read below
            // ends at eof, and eof is every copy of it being closed.
            drop(writer);

            let mut bytes = Vec::new();
            let mut chunk = [0; 4096];
            loop {
                if !wait_readable(reader.as_raw_fd(), READ_TIMEOUT_MS) {
                    return None;
                }
                match reader.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => bytes.extend_from_slice(&chunk[..n]),
                    Err(e) if e.kind() == ErrorKind::Interrupted => (),
                    Err(_) => return None,
                }
            }

            String::from_utf8(bytes).ok()
        }
    }

    /// A request, header and all. The size counts the header, so it is written
    /// once the arguments are in.
    fn message(object: u32, opcode: u16, args: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + args.len());
        out.extend_from_slice(&object.to_ne_bytes());
        out.extend_from_slice(&opcode.to_ne_bytes());
        out.extend_from_slice(&((8 + args.len()) as u16).to_ne_bytes());
        out.extend_from_slice(args);
        out
    }

    fn uint(value: u32) -> Vec<u8> {
        value.to_ne_bytes().to_vec()
    }

    fn object(id: u32) -> Vec<u8> {
        uint(id)
    }

    fn new_id(id: u32) -> Vec<u8> {
        uint(id)
    }

    /// Length including the trailing nul, the bytes, then padding to the next
    /// multiple of four.
    fn string(value: &str) -> Vec<u8> {
        let len = value.len() + 1;
        let mut out = Vec::with_capacity(4 + len.next_multiple_of(4));
        out.extend_from_slice(&(len as u32).to_ne_bytes());
        out.extend_from_slice(value.as_bytes());
        out.push(0);
        out.resize(4 + len.next_multiple_of(4), 0);
        out
    }

    /// Reads the arguments of one event back out, in the order the interface
    /// declares them.
    struct Args<'a> {
        bytes: &'a [u8],
        at: usize,
    }

    impl<'a> Args<'a> {
        fn new(bytes: &'a [u8]) -> Self {
            Self { bytes, at: 0 }
        }

        fn uint(&mut self) -> Option<u32> {
            let end = self.at + 4;
            let value = u32::from_ne_bytes(self.bytes.get(self.at..end)?.try_into().ok()?);
            self.at = end;
            Some(value)
        }

        fn string(&mut self) -> Option<&'a str> {
            let len = self.uint()? as usize;
            let text = self.bytes.get(self.at..self.at + len.checked_sub(1)?)?;
            self.at += len.next_multiple_of(4);
            std::str::from_utf8(text).ok()
        }
    }

    // What follows is the part std does not reach: passing a file descriptor
    // over a socket, and waiting on one with a timeout.

    #[repr(C)]
    struct IoVec {
        base: *mut u8,
        len: usize,
    }

    #[repr(C)]
    struct MsgHdr {
        name: *mut u8,
        namelen: u32,
        iov: *const IoVec,
        iovlen: usize,
        control: *mut u8,
        controllen: usize,
        flags: i32,
    }

    #[repr(C)]
    struct CmsgHdr {
        len: usize,
        level: i32,
        kind: i32,
    }

    #[repr(C)]
    struct PollFd {
        fd: RawFd,
        events: i16,
        revents: i16,
    }

    unsafe extern "C" {
        fn sendmsg(fd: RawFd, msg: *const MsgHdr, flags: i32) -> isize;
        fn recvmsg(fd: RawFd, msg: *mut MsgHdr, flags: i32) -> isize;
        fn poll(fds: *mut PollFd, count: u64, timeout: i32) -> i32;
    }

    const SOL_SOCKET: i32 = 1;
    const SCM_RIGHTS: i32 = 1;
    const POLLIN: i16 = 1;

    /// Sends `bytes` along with `fd`, which is the only way the other side can
    /// be given something to write into. The descriptor travels beside the
    /// bytes as a control message, not in them.
    fn send_with_fd(socket: RawFd, bytes: &[u8], fd: RawFd) -> Option<()> {
        let iov = IoVec {
            base: bytes.as_ptr().cast_mut(),
            len: bytes.len(),
        };

        // One descriptor, in a buffer aligned like the header that precedes it.
        let mut control = [0u64; 3];
        let space = size_of::<CmsgHdr>() + size_of::<RawFd>().next_multiple_of(size_of::<usize>());

        unsafe {
            let header = control.as_mut_ptr() as *mut CmsgHdr;
            header.write(CmsgHdr {
                len: size_of::<CmsgHdr>() + size_of::<RawFd>(),
                level: SOL_SOCKET,
                kind: SCM_RIGHTS,
            });
            header.add(1).cast::<RawFd>().write(fd);

            let msg = MsgHdr {
                name: std::ptr::null_mut(),
                namelen: 0,
                iov: &iov,
                iovlen: 1,
                control: control.as_mut_ptr().cast(),
                controllen: space,
                flags: 0,
            };

            (sendmsg(socket, &msg, 0) == bytes.len() as isize).then_some(())
        }
    }

    /// Whether `fd` has something to read within `timeout` milliseconds.
    fn wait_readable(fd: RawFd, timeout: i32) -> bool {
        let mut poll_fd = PollFd {
            fd,
            events: POLLIN,
            revents: 0,
        };
        unsafe { poll(&mut poll_fd, 1, timeout) == 1 }
    }
}
