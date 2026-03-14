#![allow(missing_docs, clippy::disallowed_types)]
use std::{
    collections::HashMap,
    hash::BuildHasher,
    io::{Result, Write},
};

use crate::serial::{PrefixedWrite, WriteTo};

impl WriteTo for bool {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        u8::from(*self).write(writer)?;
        Ok(())
    }
}

impl WriteTo for u8 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl WriteTo for u16 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl WriteTo for u32 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl WriteTo for u64 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl WriteTo for i8 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl WriteTo for i16 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl WriteTo for i32 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl WriteTo for i64 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl WriteTo for f32 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl WriteTo for f64 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl<T: WriteTo> WriteTo for Option<T> {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        if let Some(value) = self {
            true.write(writer)?;
            value.write(writer)
        } else {
            false.write(writer)
        }
    }
}

impl<T: WriteTo, const N: usize> WriteTo for [T; N] {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        for i in self {
            i.write(writer)?;
        }
        Ok(())
    }
}

impl<T: WriteTo, Z: WriteTo> WriteTo for (T, Z) {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.0.write(writer)?;
        self.1.write(writer)
    }
}

#[allow(missing_docs)]
impl<K: WriteTo, V: WriteTo, S: BuildHasher> WriteTo for HashMap<K, V, S> {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        (self.len() as u32).write(writer)?;
        for (key, value) in self {
            key.write(writer)?;
            value.write(writer)?;
        }
        Ok(())
    }
}

impl<T: WriteTo> WriteTo for Vec<T> {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.write_prefixed::<u32>(writer)
    }
}
