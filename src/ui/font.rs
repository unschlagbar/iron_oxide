use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

pub struct Font {
    data: [(u16, u16, u16); 256],
    pub height: u16,
}

impl Font {
    pub fn parse(path: PathBuf) -> Self {
        let mut buf = [0; 1536];
        File::open(path).unwrap().read_exact(&mut buf).unwrap();
        Self {
            data: unsafe { *buf.as_ptr().cast() },
            height: 8,
        }
    }

    pub fn parse_from_bytes(data: &[u8]) -> Self {
        let mut buf = [0; 1536];
        buf[..data.len()].copy_from_slice(data);
        Self {
            data: unsafe { *buf.as_ptr().cast() },
            height: 8,
        }
    }

    pub fn get_uv(&self, char: char) -> (u16, u16, u16) {
        let mut char = char as u32;
        if char < 32 {
            char = 64;
        }

        let i = char as usize - 32;

        self.data[i]
    }

    pub fn get_width(&self, char: char) -> u16 {
        let mut char = char as u32;
        if char < 32 {
            char = 64;
        }

        let i = char as usize - 32;

        self.data[i].2
    }
}

impl Debug for Font {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Font").finish()
    }
}
