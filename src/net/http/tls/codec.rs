//! Reading and writing the byte layout TLS uses for its structures.
//!
//! TLS encodes everything big-endian, with variable-length vectors prefixed
//! by a length whose own width is fixed by the spec at each site. Getting a
//! prefix width wrong silently shifts every field after it, so the readers
//! here are strict: a truncated or overlong buffer is an error, never a
//! partial value.

use super::TlsError;

pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8], TlsError> {
        if self.remaining() < n {
            return Err(TlsError::Decode("truncated message"));
        }
        let out = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    pub fn u8(&mut self) -> Result<u8, TlsError> {
        Ok(self.take(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16, TlsError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    pub fn u24(&mut self) -> Result<u32, TlsError> {
        let b = self.take(3)?;
        Ok(u32::from_be_bytes([0, b[0], b[1], b[2]]))
    }

    /// A vector whose length is carried in a one-byte prefix.
    pub fn vec8(&mut self) -> Result<&'a [u8], TlsError> {
        let n = self.u8()? as usize;
        self.take(n)
    }

    /// A vector whose length is carried in a two-byte prefix.
    pub fn vec16(&mut self) -> Result<&'a [u8], TlsError> {
        let n = self.u16()? as usize;
        self.take(n)
    }

    /// A vector whose length is carried in a three-byte prefix.
    pub fn vec24(&mut self) -> Result<&'a [u8], TlsError> {
        let n = self.u24()? as usize;
        self.take(n)
    }

    /// Errors unless everything has been consumed. Trailing bytes mean the
    /// message was not what we thought it was.
    pub fn expect_empty(&self) -> Result<(), TlsError> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(TlsError::Decode("trailing bytes in message"))
        }
    }
}

#[derive(Default)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bytes(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
    }

    pub fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    /// Writes a length-prefixed block. The length is patched in afterwards,
    /// so callers never have to compute sizes up front — the usual source of
    /// off-by-one bugs in hand-written encoders.
    pub fn vec8(&mut self, f: impl FnOnce(&mut Self)) {
        let at = self.buf.len();
        self.buf.push(0);
        f(self);
        let len = self.buf.len() - at - 1;
        self.buf[at] = len as u8;
    }

    pub fn vec16(&mut self, f: impl FnOnce(&mut Self)) {
        let at = self.buf.len();
        self.buf.extend_from_slice(&[0, 0]);
        f(self);
        let len = (self.buf.len() - at - 2) as u16;
        self.buf[at..at + 2].copy_from_slice(&len.to_be_bytes());
    }

    pub fn vec24(&mut self, f: impl FnOnce(&mut Self)) {
        let at = self.buf.len();
        self.buf.extend_from_slice(&[0, 0, 0]);
        f(self);
        let len = (self.buf.len() - at - 3) as u32;
        self.buf[at..at + 3].copy_from_slice(&len.to_be_bytes()[1..]);
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.buf
    }
}
