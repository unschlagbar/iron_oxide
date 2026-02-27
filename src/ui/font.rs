use std::fmt;

use crate::primitives::Vec2;

pub struct Font {
    data: Box<[RawGlyph; 256]>,
    /// Base size of the font
    pub size: f32,
    /// Distance between two lines
    pub line_height: f32,
    /// Baseline hieght of the font
    pub base: f32,
    /// Whether the font is a bitmap font
    pub bitmap: bool,
}

impl Font {
    pub fn parse_bitmap_from_bytes(data: &[u8]) -> Self {
        assert!(data.len() >= 256 * 6);

        let mut glyphs = [RawGlyph::default(); 256];

        for i in 0..256 {
            let base = i * 6;

            let u = u16::from_le_bytes([data[base], data[base + 1]]);
            let v = u16::from_le_bytes([data[base + 2], data[base + 3]]);
            let w = u16::from_le_bytes([data[base + 4], data[base + 5]]);

            glyphs[i] = RawGlyph {
                size: Vec2::new(w, 8),
                offset: Vec2::new(0.0, 0.0),
                pos: Vec2::new(u, v),
                advance: w as f32,
            };
        }

        Self {
            data: Box::new(glyphs),
            size: 8.0,
            line_height: 8.0,
            base: 8.0,
            //distance_range: 0,
            bitmap: true,
        }
    }

    pub fn parse_msdf_from_bytes(data: &[u8]) -> Self {
        let text = str::from_utf8(data).unwrap();

        let size = extract_number(text, "\"size\"") as f32;
        let line_height = extract_number(text, "\"lineHeight\"") as f32;
        let base = extract_number(text, "\"base\"") as f32;

        let mut glyphs = [RawGlyph::default(); 256];

        let chars_start = text.find("\"chars\"").unwrap();
        let chars_section = &text[chars_start..];

        let mut rest = chars_section;

        while let Some(pos) = rest.find("\"id\"") {
            rest = &rest[pos..];

            let id = extract_number(rest, "\"id\"");

            let width = extract_number(rest, "\"width\"");
            let height = extract_number(rest, "\"height\"");
            let xoffset = extract_number(rest, "\"xoffset\"") as f32;
            let yoffset = extract_number(rest, "\"yoffset\"") as f32;
            let advance = extract_number(rest, "\"xadvance\"") as f32;
            let x = extract_number(rest, ",\"x\"");
            let y = extract_number(rest, ",\"y\"");

            if id < 256 {
                glyphs[id as usize] = RawGlyph {
                    size: Vec2::new(width, height),
                    offset: Vec2::new(xoffset, yoffset),
                    pos: Vec2::new(x, y),
                    advance,
                };
            }

            rest = &rest[5..];
        }

        Self {
            data: Box::new(glyphs),
            size: size,
            line_height,
            base,
            bitmap: false,
        }
    }

    pub fn get_glyph(&self, char: char) -> RawGlyph {
        let char = Self::char_index(char);
        let i = char as usize;
        *self.data.get(i).unwrap_or(&RawGlyph::default())
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

#[derive(Default, Debug, Clone, Copy)]
pub struct RawGlyph {
    /// Size of the glyph
    pub size: Vec2<u16>,
    /// Offset of the glyph from the baseline for placing
    pub offset: Vec2<f32>,
    /// UV coordinates in the font atlas
    pub pos: Vec2<u16>,
    /// Horizontal advance after placing this glyph
    pub advance: f32,
}

fn extract_number(source: &str, key: &str) -> u16 {
    let start = source.find(key).unwrap();
    let sub = &source[start + key.len()..];

    let colon = sub.find(':').unwrap();
    let sub = &sub[colon + 1..];

    let end = sub.find(|c: char| !c.is_numeric()).unwrap();
    sub[..end].trim().parse().unwrap_or_default()
}
