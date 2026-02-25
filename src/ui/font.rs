use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

pub struct Font {
    data: Box<[(u16, u16, u16); 256]>,
    pub height: u16,
    pub bitmap: bool,
}

impl Font {
    pub fn parse(path: PathBuf, bitmap: bool) -> Self {
        let mut buf = [0; 1536];
        File::open(path).unwrap().read_exact(&mut buf).unwrap();
        let buf: [(u16, u16, u16); 256] = unsafe { *buf.as_ptr().cast() };
        Self {
            data: buf.into(),
            height: 8,
            bitmap,
        }
    }

    pub fn parse_from_bytes(data: &[u8], bitmap: bool) -> Self {
        let mut buf = [0; 1536];
        buf[..data.len()].copy_from_slice(data);
        let buf: [(u16, u16, u16); 256] = unsafe { *buf.as_ptr().cast() };
        Self {
            data: buf.into(),
            height: 8,
            bitmap,
        }
    }

    pub fn get_uv(&self, char: char) -> (u16, u16, u16) {
        let char = Self::char_index(char);
        let i = char as usize;
        *self.data.get(i).unwrap_or(&(0, 0, 0))
    }

    pub fn get_width(&self, char: char) -> u16 {
        self.get_uv(char).2
    }

    pub fn char_index(char: char) -> u32 {
        let mut index = char as u32;
        if index < 32 {
            index = 64;
        }
        match char {
            'ü' => 8 * 16 + 1,
            'ä' => 8 * 16 + 4,
            'ö' => 9 * 16 + 4,

            'Ü' => 9 * 16 + 10,
            'Ä' => 8 * 16 + 14,
            'Ö' => 9 * 16 + 9,

            'ß' => 11,

            _ => index,
        }
    }
}

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Font").finish()
    }
}
