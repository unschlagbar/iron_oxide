use std::fmt::Debug;
use std::path::PathBuf;
use std::io::Read;
use std::fs::File;


pub struct Font {
    data: [u16; 1024],
}

impl Font {
    pub fn parse(path: PathBuf) -> Self {
        let mut buf: [u8; 2048] = [0; 2048];
        File::open(path).unwrap().read(&mut buf).unwrap();
        Self { data: unsafe {*(buf.as_ptr() as *const [u16; 1024])} }
    }

    pub fn parse_from_bytes(data: &[u8]) -> Self {
        let mut buf: [u8; 2048] = [0; 2048];

        for i in 0..data.len() {
            buf[i] = data[i];
        }
        Self { data: unsafe {*(buf.as_ptr() as *const [u16; 1024])} }
    }

    pub fn get_data(&self, char: u8) -> (u16, u16, u16, u16) {
        if char < 32 {
            todo!();
        }

        let i = (char as usize - 32) * 4;

        (self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3])
    }
}

impl Debug for Font {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Font").finish()
    }
}