use std::{
    io::{Cursor, Read, Result},
    mem::MaybeUninit,
};

use crate::serial::ReadFrom;

impl ReadFrom for bool {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let byte = u8::read(data)?;
        Ok(byte == 1)
    }
}

impl ReadFrom for u8 {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
}

impl ReadFrom for u16 {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
}

impl ReadFrom for u32 {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
}

impl ReadFrom for u64 {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
}

impl ReadFrom for i8 {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
}

impl ReadFrom for i16 {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
}

impl ReadFrom for i32 {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
}

impl ReadFrom for i64 {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
}

impl ReadFrom for f32 {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
}

impl ReadFrom for f64 {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
}

impl<T: ReadFrom> ReadFrom for Option<T> {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        if bool::read(data)? {
            Ok(Some(T::read(data)?))
        } else {
            Ok(None)
        }
    }
}

impl<T: ReadFrom, const N: usize> ReadFrom for [T; N] {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let mut buf = [const { MaybeUninit::<T>::uninit() }; N];

        for i in 0..N {
            match T::read(data) {
                Ok(value) => {
                    buf[i].write(value);
                }
                Err(e) => {
                    for slot in &mut buf[..i] {
                        // SAFETY: the elements before `i` were written above.
                        unsafe { slot.assume_init_drop() };
                    }
                    return Err(e);
                }
            }
        }

        // SAFETY: every element is written, and `[MaybeUninit<T>; N]` has the
        // layout of `[T; N]`.
        Ok(unsafe { buf.as_ptr().cast::<[T; N]>().read() })
    }
}
