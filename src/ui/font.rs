//! Fonts in the Artery Atlas Font format (`.arfont`) written by msdf-atlas-gen,
//! see <https://github.com/Chlumsky/artery-font-format>.

use std::{
    error, fmt,
    io::{self, Cursor},
    rc::Rc,
    str,
};

use crate::{
    primitives::Vec2,
    serial::{ReadFrom, ReadLen},
    ui::materials::MatType,
};

const HEADER_TAG: &[u8; 16] = b"ARTERY/FONT\0\0\0\0\0";
const HEADER_MAGIC: u32 = 0x4d27_6a5c;
const FOOTER_MAGIC: u32 = 0x55cc_b363;
const REAL_F32: u32 = 0x14;
const HEADER_LEN: usize = 112;
const FOOTER_LEN: usize = 32;

const CP_UNICODE: u32 = 1;
const ORIENTATION_BOTTOM_UP: i32 = -1;
const IMAGE_SRGB: u32 = 1;
const IMAGE_LINEAR_MASK: u32 = 2;

const MISSING: RawGlyph = RawGlyph {
    left: 0.0,
    top: 0.0,
    right: 0.0,
    bottom: 0.0,
    atlas_start: Vec2::zero(),
    atlas_end: Vec2::zero(),
    advance: 0.0,
};

/// One font variant of an atlas. All metrics are in em with y pointing down,
/// so `ascender` is negative and `descender` positive.
pub struct Font {
    pub name: String,
    direct: [RawGlyph; 256],
    /// Sorted, parallel to `glyphs`; only codepoints `>= DIRECT`.
    codepoints: Vec<u32>,
    glyphs: Vec<RawGlyph>,
    /// Sorted by `left << 32 | right`.
    kerning: Vec<(u64, f32)>,
    /// Distance between two lines
    pub line_height: f32,
    pub ascender: f32,
    pub descender: f32,
    /// Width of the distance field in atlas pixels.
    pub distance_range: f32,
    /// Whether the font is a bitmap font
    pub bitmap: bool,
}

impl Font {
    pub fn get_glyph(&self, char: char) -> &RawGlyph {
        let c = char as u32;
        if (c as usize) < 256 {
            &self.direct[c as usize]
        } else {
            match self.codepoints.binary_search(&c) {
                Ok(i) => &self.glyphs[i],
                Err(_) => &MISSING,
            }
        }
    }

    pub fn has_glyph(&self, char: char) -> bool {
        let c = char as u32;
        if (c as usize) < 256 {
            self.direct[c as usize].advance != 0.0
        } else {
            self.codepoints.binary_search(&c).is_ok()
        }
    }

    /// Advance correction in em between `left` and the `right` that follows it.
    pub fn kerning(&self, left: char, right: char) -> f32 {
        if self.kerning.is_empty() {
            return 0.0;
        }
        let key = (left as u64) << 32 | right as u64;
        match self.kerning.binary_search_by_key(&key, |&(k, _)| k) {
            Ok(i) => self.kerning[i].1,
            Err(_) => 0.0,
        }
    }

    pub fn material(&self) -> MatType {
        if self.bitmap {
            MatType::Bitmap
        } else {
            MatType::MSDF
        }
    }
}

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Font").field("name", &self.name).finish()
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct RawGlyph {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    /// Atlas pixels, origin at the top left of the image.
    pub atlas_start: Vec2<f32>,
    pub atlas_end: Vec2<f32>,
    /// Horizontal advance after placing this glyph
    pub advance: f32,
}

/// A parsed `.arfont`: its font variants and the atlas images they sample.
/// Images borrow their encoded bytes from the input.
pub struct ArteryFont<'a> {
    pub fonts: Vec<Rc<Font>>,
    pub images: Vec<FontImage<'a>>,
}

impl<'a> ArteryFont<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, FontError> {
        let len = data.len();
        if len < HEADER_LEN + FOOTER_LEN {
            return Err(FontError::Truncated);
        }

        let c = &mut Cursor::new(&data[len - FOOTER_LEN..]);
        let _salt = u32::read(c)?;
        let footer_magic = u32::read(c)?;
        let _reserved = <[u32; 4]>::read(c)?;
        let total_len = u32::read(c)?;
        let stored_checksum = u32::read(c)?;
        if footer_magic != FOOTER_MAGIC {
            return Err(FontError::NotArteryFont);
        }
        if total_len as usize != len {
            return Err(FontError::Truncated);
        }
        if checksum(&data[..len - 4]) != stored_checksum {
            return Err(FontError::Checksum);
        }

        let c = &mut Cursor::new(&data[..len - FOOTER_LEN]);
        if &<[u8; 16]>::read(c)? != HEADER_TAG || u32::read(c)? != HEADER_MAGIC {
            return Err(FontError::NotArteryFont);
        }
        let _version = u32::read(c)?;
        let _flags = u32::read(c)?;
        if u32::read(c)? != REAL_F32 {
            return Err(FontError::Unsupported("real type other than f32"));
        }
        let _reserved = <[u32; 4]>::read(c)?;
        let _metadata_format = u32::read(c)?;
        let metadata_len = u32::read(c)?;
        let variant_count = u32::read(c)?;
        let variants_len = u32::read(c)? as u64;
        let image_count = u32::read(c)?;
        let images_len = u32::read(c)? as u64;
        let _appendix_count = u32::read(c)?;
        let _appendices_len = u32::read(c)?;
        let _reserved = <[u32; 8]>::read(c)?;
        <()>::read_len(c, stored_len(metadata_len))?;
        let variants_start = c.position();

        // Images come after the variants, but a glyph's atlas position depends on
        // the orientation of the image it is in.
        c.set_position(variants_start + variants_len);
        let images = (0..image_count)
            .map(|_| FontImage::read(c))
            .collect::<Result<Vec<_>, _>>()?;
        if c.position() != variants_start + variants_len + images_len {
            return Err(FontError::Corrupt);
        }

        c.set_position(variants_start);
        let fonts = (0..variant_count)
            .map(|_| Font::read(c, &images).map(Rc::new))
            .collect::<Result<Vec<_>, _>>()?;
        if c.position() != variants_start + variants_len {
            return Err(FontError::Corrupt);
        }

        Ok(Self { fonts, images })
    }

    pub fn font(&self, name: &str) -> Option<Rc<Font>> {
        self.fonts.iter().find(|f| f.name == name).cloned()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageEncoding {
    /// Uncompressed rows of `channels` bytes per pixel.
    Raw,
    Bmp,
    Tiff,
    Png,
    Tga,
    Unknown(u32),
}

#[derive(Debug, Clone, Copy)]
pub struct FontImage<'a> {
    pub encoding: ImageEncoding,
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    /// Only meaningful for [`ImageEncoding::Raw`]; encoded formats are top-down.
    pub bottom_up: bool,
    pub data: &'a [u8],
}

impl<'a> FontImage<'a> {
    fn read(c: &mut Cursor<&'a [u8]>) -> Result<Self, FontError> {
        let _flags = u32::read(c)?;
        let encoding = match u32::read(c)? {
            1 => ImageEncoding::Raw,
            4 => ImageEncoding::Bmp,
            5 => ImageEncoding::Tiff,
            8 => ImageEncoding::Png,
            9 => ImageEncoding::Tga,
            other => ImageEncoding::Unknown(other),
        };
        let width = u32::read(c)?;
        let height = u32::read(c)?;
        let channels = u32::read(c)?;
        let _pixel_format = u32::read(c)?;
        let _image_type = u32::read(c)?;
        let _row_length = u32::read(c)?;
        let orientation = i32::read(c)?;
        let bottom_up = encoding == ImageEncoding::Raw && orientation == ORIENTATION_BOTTOM_UP;
        let _child_images = u32::read(c)?;
        let _texture_flags = u32::read(c)?;
        let _reserved = <[u32; 3]>::read(c)?;
        let metadata_len = u32::read(c)?;
        let data_len = u32::read(c)?;
        <()>::read_len(c, stored_len(metadata_len))?;
        let data_len = data_len as usize;
        let data = <&[u8]>::read_len(c, data_len)?;
        <()>::read_len(c, data_len.next_multiple_of(4) - data_len)?;

        Ok(Self {
            encoding,
            width,
            height,
            channels,
            bottom_up,
            data,
        })
    }
}

impl Font {
    /// One variant, whose glyphs sit in `images`.
    fn read(c: &mut Cursor<&[u8]>, images: &[FontImage<'_>]) -> Result<Self, FontError> {
        let _flags = u32::read(c)?;
        let _weight = u32::read(c)?;
        if u32::read(c)? != CP_UNICODE {
            return Err(FontError::Unsupported(
                "glyphs not keyed by unicode codepoint",
            ));
        }
        let image_type = u32::read(c)?;
        let _fallback_variant = u32::read(c)?;
        let _fallback_glyph = u32::read(c)?;
        let _reserved = <[u32; 6]>::read(c)?;

        let _font_size = f32::read(c)?;
        let distance_range = f32::read(c)?;
        let em = f32::read(c)?;
        let ascender = f32::read(c)?;
        let descender = f32::read(c)?;
        let line_height = f32::read(c)?;
        let _underline_y = f32::read(c)?;
        let _underline_thickness = f32::read(c)?;
        let _distance_range_middle = f32::read(c)?;
        let _reserved = <[f32; 23]>::read(c)?;
        let s = if em > 0.0 { 1.0 / em } else { 1.0 };

        let name_len = u32::read(c)?;
        let metadata_len = u32::read(c)?;
        let glyph_count = u32::read(c)?;
        let kern_count = u32::read(c)?;
        let name = String::read_len(c, name_len as usize)?;
        <()>::read_len(c, stored_len(name_len) - name_len as usize)?;
        <()>::read_len(c, stored_len(metadata_len))?;

        let mut direct = [MISSING; 256];
        let mut wide = Vec::new();
        for _ in 0..glyph_count {
            let codepoint = u32::read(c)?;
            let image = u32::read(c)?;
            let [left, bottom, right, top] = <[f32; 4]>::read(c)?;
            let [atlas_left, atlas_bottom, atlas_right, atlas_top] = <[f32; 4]>::read(c)?;
            let advance = f32::read(c)?;
            let _vertical_advance = f32::read(c)?;

            let (atlas_start, atlas_end) = if atlas_right > atlas_left {
                let image = images.get(image as usize).ok_or(FontError::Corrupt)?;
                // Bounds are y-up; flip them for an image whose first row is its top.
                let (start_y, end_y) = if image.bottom_up {
                    (atlas_top, atlas_bottom)
                } else {
                    let height = image.height as f32;
                    (height - atlas_top, height - atlas_bottom)
                };
                (
                    Vec2::new(atlas_left, start_y),
                    Vec2::new(atlas_right, end_y),
                )
            } else {
                (Vec2::zero(), Vec2::zero())
            };

            let glyph = RawGlyph {
                left: left * s,
                top: -top * s,
                right: right * s,
                bottom: -bottom * s,
                atlas_start,
                atlas_end,
                advance: advance * s,
            };

            if (codepoint as usize) < 256 {
                direct[codepoint as usize] = glyph;
            } else {
                wide.push((codepoint, glyph));
            }
        }
        wide.sort_unstable_by_key(|&(c, _)| c);
        let (codepoints, glyphs) = wide.into_iter().unzip();

        let mut kerning = Vec::new();
        for _ in 0..kern_count {
            let key = (u32::read(c)? as u64) << 32 | u32::read(c)? as u64;
            kerning.push((key, f32::read(c)? * s));
            let _vertical_advance = f32::read(c)?;
        }
        kerning.sort_unstable_by_key(|&(k, _)| k);

        Ok(Self {
            name,
            direct,
            codepoints,
            glyphs,
            kerning,
            line_height: line_height * s,
            ascender: -ascender * s,
            descender: -descender * s,
            distance_range,
            bitmap: matches!(image_type, IMAGE_SRGB | IMAGE_LINEAR_MASK),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontError {
    Truncated,
    NotArteryFont,
    Checksum,
    /// A length or offset inside the file does not add up.
    Corrupt,
    Unsupported(&'static str),
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => f.write_str("arfont: file is truncated"),
            Self::NotArteryFont => f.write_str("arfont: not an artery font file"),
            Self::Checksum => f.write_str("arfont: checksum mismatch"),
            Self::Corrupt => f.write_str("arfont: inconsistent section lengths"),
            Self::Unsupported(what) => write!(f, "arfont: unsupported {what}"),
        }
    }
}

impl error::Error for FontError {}

impl From<io::Error> for FontError {
    fn from(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::InvalidData => Self::Corrupt,
            _ => Self::Truncated,
        }
    }
}

/// Bytes a string of `len` takes in the file: nul terminated and padded to 4,
/// nothing at all when empty.
fn stored_len(len: u32) -> usize {
    match len {
        0 => 0,
        len => (len as usize + 1).next_multiple_of(4),
    }
}

/// CRC-32 (IEEE) without the final inversion, as the format stores it.
fn checksum(data: &[u8]) -> u32 {
    !crc32fast::hash(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serial::WriteTo;

    /// A minimal arfont: one variant with the given glyphs and kern pairs, one
    /// 64x32 image.
    fn encode(
        glyphs: &[[f32; 10]],
        codepoints: &[u32],
        kern: &[(u32, u32, f32)],
        raw_bottom_up: bool,
    ) -> Vec<u8> {
        const VARIANT_HEADER_LEN: usize = 192;
        const IMAGE_HEADER_LEN: usize = 64;
        const GLYPH_LEN: usize = 48;
        const KERN_PAIR_LEN: usize = 16;

        let name = b"Test";
        let image_data = [7u8; 6];
        let pad = |len: usize| (len + 3) & !3;

        let variants_len = VARIANT_HEADER_LEN
            + pad(name.len() + 1)
            + glyphs.len() * GLYPH_LEN
            + kern.len() * KERN_PAIR_LEN;
        let images_len = IMAGE_HEADER_LEN + pad(image_data.len());

        let mut out = Vec::new();
        let o = &mut out;
        let u32s = |o: &mut Vec<u8>, values: &[u32]| {
            for v in values {
                v.write(o).unwrap();
            }
        };

        o.extend_from_slice(HEADER_TAG);
        u32s(o, &[HEADER_MAGIC, 1, 0, REAL_F32, 0, 0, 0, 0]);
        u32s(
            o,
            &[0, 0, 1, variants_len as u32, 1, images_len as u32, 0, 0],
        );
        u32s(o, &[0; 8]);
        assert_eq!(o.len(), HEADER_LEN);

        u32s(o, &[0, 400, CP_UNICODE, 6, 0, 0, 0, 0, 0, 0, 0, 0]);
        let mut metrics: [f32; 32] = [0.0; 32];
        metrics[..6].copy_from_slice(&[40.0, 4.0, 2.0, 1.8, -0.4, 2.4]);
        for m in metrics {
            m.write(o).unwrap();
        }
        u32s(
            o,
            &[name.len() as u32, 0, glyphs.len() as u32, kern.len() as u32],
        );
        o.extend_from_slice(name);
        o.resize(pad(o.len() + 1), 0);
        for (g, &cp) in glyphs.iter().zip(codepoints) {
            u32s(o, &[cp, 0]);
            for v in g {
                v.write(o).unwrap();
            }
        }
        for &(a, b, advance) in kern {
            u32s(o, &[a, b]);
            advance.write(o).unwrap();
            0u32.write(o).unwrap(); // vertical advance
        }

        let (encoding, orientation) = if raw_bottom_up { (1, -1) } else { (8, 0) };
        u32s(o, &[0, encoding, 64, 32, 4, 8, 6, 0]);
        (orientation as i32).write(o).unwrap();
        u32s(o, &[0, 0, 0, 0, 0, 0, image_data.len() as u32]);
        o.extend_from_slice(&image_data);
        o.resize(pad(o.len()), 0);

        let total = o.len() + FOOTER_LEN;
        u32s(o, &[0, FOOTER_MAGIC, 0, 0, 0, 0, total as u32]);
        let crc = super::checksum(o);
        crc.write(o).unwrap();
        out
    }

    const A: [f32; 10] = [0.1, -0.2, 1.1, 1.6, 3.5, 2.5, 14.5, 20.5, 1.2, 0.0];

    #[test]
    fn parses_metrics_and_flips_to_y_down() {
        let file = encode(
            &[A, A],
            &[b'A' as u32, 0x2014],
            &[(65, 0x2014, -0.2)],
            false,
        );
        let atlas = ArteryFont::parse(&file).unwrap();
        let font = &atlas.fonts[0];

        assert_eq!(font.name, "Test");
        assert_eq!(font.distance_range, 4.0);
        assert_eq!(font.ascender, -0.9);
        assert_eq!(font.descender, 0.2);
        assert_eq!(font.line_height, 1.2);

        for c in ['A', '—'] {
            let g = font.get_glyph(c);
            assert_eq!((g.left, g.right, g.advance), (0.05, 0.55, 0.6));
            assert_eq!((g.top, g.bottom), (-0.8, 0.1));
            assert_eq!((g.atlas_start.x, g.atlas_start.y), (3.5, 32.0 - 20.5));
            assert_eq!((g.atlas_end.x, g.atlas_end.y), (14.5, 32.0 - 2.5));
        }
        assert_eq!(font.get_glyph('°').advance, 0.0);
        assert!(!font.has_glyph('°') && font.has_glyph('—'));
        assert_eq!(font.kerning('A', '—'), -0.1);
        assert_eq!(font.kerning('—', 'A'), 0.0);

        assert_eq!(atlas.images[0].encoding, ImageEncoding::Png);
        assert_eq!(atlas.images[0].data, &[7; 6]);
    }

    #[test]
    fn raw_bottom_up_image_is_not_flipped() {
        let file = encode(&[A], &[b'A' as u32], &[], true);
        let atlas = ArteryFont::parse(&file).unwrap();
        let g = atlas.fonts[0].get_glyph('A');
        assert_eq!((g.atlas_start.y, g.atlas_end.y), (20.5, 2.5));
    }

    #[test]
    fn rejects_damage() {
        let file = encode(&[A], &[b'A' as u32], &[], false);
        let mut flipped = file.clone();
        flipped[HEADER_LEN + 70] ^= 1;
        assert_eq!(ArteryFont::parse(&flipped).err(), Some(FontError::Checksum));
        assert_eq!(
            ArteryFont::parse(&file[..file.len() - 4]).err(),
            Some(FontError::NotArteryFont)
        );
        assert_eq!(
            ArteryFont::parse(&file[..40]).err(),
            Some(FontError::Truncated)
        );
    }
}
