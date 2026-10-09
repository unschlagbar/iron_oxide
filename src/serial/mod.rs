use std::io::{Cursor, Result, Write};

/// A module for reading data of a given length.
mod len_read;
/// A module for reading prefixed data.
mod prefixed_read;
/// A module for writing prefixed data.
mod prefixed_write;
/// A module for reading data.
mod read;
/// A module for writing data.
mod write;

const DEFAULT_BOUND: usize = i16::MAX as _;

/// A trait for reading data from a cursor.
pub trait ReadFrom: Sized {
    /// Reads data from a cursor.
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self>;
}

/// A trait for reading data whose length is stored elsewhere. What is read
/// borrows from the cursor's slice instead of being copied out of it.
pub trait ReadLen<'a>: Sized {
    /// Reads `len` bytes from a cursor.
    fn read_len(data: &mut Cursor<&'a [u8]>, len: usize) -> Result<Self>;
}

/// A trait for writing data to a writer.
pub trait WriteTo {
    /// Writes data to a writer.
    fn write(&self, writer: &mut impl Write) -> Result<()>;
}

/// A trait for reading prefixed data from a cursor.
pub trait PrefixedRead: Sized {
    /// Reads prefixed data from a cursor with a bound.
    fn read_prefixed_bound<P: TryInto<usize> + ReadFrom>(
        data: &mut Cursor<&[u8]>,
        bound: usize,
    ) -> Result<Self>;

    /// Reads prefixed data from a cursor.
    fn read_prefixed<P: TryInto<usize> + ReadFrom>(data: &mut Cursor<&[u8]>) -> Result<Self> {
        Self::read_prefixed_bound::<P>(data, DEFAULT_BOUND)
    }
}

/// A trait for writing prefixed data to a writer.
pub trait PrefixedWrite {
    /// Writes prefixed data to a writer with a bound.
    fn write_prefixed_bound<P: TryFrom<usize> + WriteTo>(
        &self,
        writer: &mut impl Write,
        bound: usize,
    ) -> Result<()>;

    /// Writes prefixed data to a writer.
    fn write_prefixed<P: TryFrom<usize> + WriteTo>(&self, writer: &mut impl Write) -> Result<()> {
        self.write_prefixed_bound::<P>(writer, DEFAULT_BOUND)
    }
}
