use std::fmt;

use crate::primitives::Vec2;

pub struct Font {
    glyphs: [RawGlyph; 256],
    /// Distance between two lines
    pub line_height: f32,
    pub ascender: f32,
    pub descender: f32,
    /// Whether the font is a bitmap font
    pub bitmap: bool,
}

impl Font {
    //pub fn parse_bitmap_from_bytes(data: &[u8]) -> Self {
    //    assert!(data.len() >= 256 * 6);

    //   let mut glyphs = [RawGlyph::default(); 256];

    //    for i in 0..256 {
    //       let base = i * 6;

    //       let u = u16::from_le_bytes([data[base], data[base + 1]]);
    //        let v = u16::from_le_bytes([data[base + 2], data[base + 3]]);
    //        let w = u16::from_le_bytes([data[base + 4], data[base + 5]]);
    //
    //       glyphs[i] = RawGlyph {
    //            size: Vec2::new(w, 8),
    //            offset: Vec2::new(0.0, 0.0),
    //            pos: Vec2::new(u, v),
    //            advance: w as f32,
    //        };
    //    }

    //    Self {
    //        data: Box::new(glyphs),
    //       line_height: 8.0,
    //        ascender: 8.0,
    //        descender: 0.0,
    //        //distance_range: 0,
    //         bitmap: true,
    //     }
    // }

    pub fn parse_msdf_from_bytes(data: &[u8]) -> Self {
        let text = std::str::from_utf8(data).unwrap();

        let line_height = extract_float(text, "\"lineHeight\"");
        let ascender = extract_float(text, "\"ascender\"");
        let descender = extract_float(text, "\"descender\"");

        let mut glyphs = [RawGlyph::default(); 256];

        let glyphs_start = text.find("\"glyphs\"").unwrap();
        let mut rest = &text[glyphs_start..];


        while let Some(pos) = rest.find("\"unicode\"") {
            // zum Anfang des Objekts gehen
            let start = rest[..pos].rfind('{').unwrap();
            let obj = try_extract_block(&rest[start..], "{").unwrap();

            let unicode = extract_number(obj, "\"unicode\"") as usize;
            let advance = extract_float(obj, "\"advance\"");

            let plane_block = try_extract_block(obj, "\"planeBounds\"");
            let atlas_block = try_extract_block(obj, "\"atlasBounds\"");

            let (left, right, bottom, top) = if let Some(pb) = plane_block {
                (
                    extract_float(pb, "\"left\""),
                    extract_float(pb, "\"right\""),
                    extract_float(pb, "\"bottom\""),
                    extract_float(pb, "\"top\""),
                )
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };

            let (atlas_start, atlas_end) = if let Some(ab) = atlas_block {
                (
                    Vec2::new(extract_float(ab, "\"left\""), extract_float(ab, "\"top\"")),
                    Vec2::new(
                        extract_float(ab, "\"right\""),
                        extract_float(ab, "\"bottom\""),
                    ),
                )
            } else {
                (Vec2::zero(), Vec2::zero())
            };

            if unicode < 256 {
                glyphs[unicode] = RawGlyph {
                    left,
                    right,
                    bottom,
                    top,
                    atlas_start,
                    atlas_end,
                    advance,
                };
            }

            // nach diesem Objekt weitermachen
            let offset = start + obj.len();
            rest = &rest[offset..];
        }

        Self {
            glyphs,
            line_height,
            ascender,
            descender,
            bitmap: false,
        }
    }

    pub fn get_glyph(&self, char: char) -> RawGlyph {
        let char = Self::char_index(char);
        let i = char as usize;
        self.glyphs.get(i).copied().unwrap_or_default()
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
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub atlas_start: Vec2<f32>,
    pub atlas_end: Vec2<f32>,
    /// Horizontal advance after placing this glyph
    pub advance: f32,
}

fn extract_number(source: &str, key: &str) -> u16 {
    let start = if let Some(key) = source.find(key) {
        key
    } else {
        return 0;
    };
    let sub = &source[start + key.len()..];

    let colon = sub.find(':').unwrap();
    let sub = &sub[colon + 1..];

    let end = sub.find(|c: char| !c.is_numeric()).unwrap();
    sub[..end].trim().parse().unwrap_or_default()
}

fn extract_float(source: &str, key: &str) -> f32 {
    let start = source.find(key).unwrap();
    let sub = &source[start + key.len()..];

    let colon = sub.find(':').unwrap();
    let sub = &sub[colon + 1..];

    let end = sub
        .find(|c: char| !(c.is_numeric() || c == '.' || c == '-'))
        .unwrap();

    sub[..end].trim().parse().unwrap()
}

fn try_extract_block<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let start = source.find(key)?;
    let sub = &source[start..];

    let open = sub.find('{')?;
    let mut depth = 1;
    let mut i = open + 1;

    while depth > 0 {
        match sub.as_bytes()[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }

    Some(&sub[open..i])
}