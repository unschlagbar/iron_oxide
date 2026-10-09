use std::{
    io::{Cursor, Error, ErrorKind, Result},
    str,
};

use crate::serial::ReadLen;

impl<'a> ReadLen<'a> for &'a [u8] {
    fn read_len(data: &mut Cursor<&'a [u8]>, len: usize) -> Result<Self> {
        let slice: &'a [u8] = data.get_ref();
        let start = data.position() as usize;
        let bytes = start
            .checked_add(len)
            .and_then(|end| slice.get(start..end))
            .ok_or(ErrorKind::UnexpectedEof)?;
        data.set_position((start + len) as u64);
        Ok(bytes)
    }
}

impl<'a> ReadLen<'a> for &'a str {
    fn read_len(data: &mut Cursor<&'a [u8]>, len: usize) -> Result<Self> {
        str::from_utf8(<&[u8]>::read_len(data, len)?)
            .map_err(|e| Error::new(ErrorKind::InvalidData, e))
    }
}

impl<'a> ReadLen<'a> for String {
    fn read_len(data: &mut Cursor<&'a [u8]>, len: usize) -> Result<Self> {
        <&str>::read_len(data, len).map(str::to_owned)
    }
}

/// Skips `len` bytes.
impl<'a> ReadLen<'a> for () {
    fn read_len(data: &mut Cursor<&'a [u8]>, len: usize) -> Result<Self> {
        <&[u8]>::read_len(data, len).map(|_| ())
    }
}
