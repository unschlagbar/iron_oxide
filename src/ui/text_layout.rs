use std::rc::Rc;

use crate::{
    primitives::Vec2,
    ui::{BuildContext, Font},
};

pub enum TextDirtyFlags {
    None,
    TextChanged,
    AddedChar,
    RemovedChar,
}

#[derive(Default)]
pub enum TextOverflow {
    /// Doesn't handle overflow
    Allow,
    #[default]
    /// Cuts text that goes out of the parent element
    Clip,
    /// Replaces overflowing text with "..."
    Ellipsis,
}

#[derive(Default)]
pub enum WhiteSpace {
    /// Collapses consecutive spaces and allows line wrapping.
    /// Default behavior for normal text content.
    Normal,

    /// Collapses consecutive spaces but prevents line wrapping.
    /// Text stays on a single line until manually broken.
    NoWrap,

    #[default]
    /// Preserves all spaces and line breaks exactly as written.
    /// No automatic wrapping.
    Pre,

    /// Preserves all spaces and line breaks, but also allows wrapping
    /// when the text exceeds the container width.
    PreWrap,

    /// Collapses multiple spaces but preserves line breaks.
    /// Allows wrapping between lines.
    PreLine,

    /// Like `PreWrap`, but allows wrapping even within sequences
    /// of spaces. Used in modern CSS for precise text editors.
    BreakSpaces,
}

impl WhiteSpace {
    pub fn newlines(&self) -> bool {
        matches!(
            self,
            WhiteSpace::Pre | WhiteSpace::PreWrap | WhiteSpace::PreLine | WhiteSpace::BreakSpaces
        )
    }

    pub fn wrap(&self) -> bool {
        !matches!(self, WhiteSpace::NoWrap | WhiteSpace::Pre)
    }

    pub fn collapses_spaces(&self) -> bool {
        matches!(
            self,
            WhiteSpace::Normal | WhiteSpace::NoWrap | WhiteSpace::PreLine
        )
    }
}

#[derive(Default)]
pub enum OverflowWrap {
    #[default]
    None,
    BreakWord,
}

pub struct TextLayout {
    pub font: Option<Rc<Font>>,
    pub font_size: f32,
    pub line_spacing: f32,
    pub overflow: TextOverflow,
    pub overflow_wrap: OverflowWrap,
    pub white_space: WhiteSpace,

    pub lines: Vec<TextLine>,
    pub glyphs: Vec<Glyph>,
    pub size: Vec2<f32>,
}

impl TextLayout {
    pub fn build(&mut self, text: &str, ctx: &mut BuildContext) {
        let container_size = ctx.available_space;

        self.glyphs.clear();
        self.glyphs.reserve(text.len());
        self.lines.clear();
        self.lines.push(TextLine::default());

        let font = self.font.as_ref().unwrap_or(&ctx.font);
        let font_size = self.font_size * ctx.scale_factor;
        let line_height = font.line_height * self.line_spacing * font_size;

        let mut width: f32 = 0.0;

        let mut cursor = Vec2::new(0.0, -font.ascender * font_size);
        let mut last_whitespace = true;
        let mut split_point = usize::MAX;

        for mut char in text.chars() {
            let whitespace = char.is_whitespace();
            let mut overflowed = false;

            if char == '\n' {
                if self.white_space.newlines() {
                    //layout.lines.push(TextLine::default());
                    self.lines.push(TextLine {
                        start: self.glyphs.len(),
                        end: self.glyphs.len(),
                        width: 0.0,
                    });

                    width = width.max(cursor.x);

                    cursor.x = 0.0;
                    cursor.y += line_height;
                    split_point = usize::MAX;
                    continue;
                } else {
                    char = ' '
                }
            }

            // Handle space collapsing
            if whitespace && last_whitespace && self.white_space.collapses_spaces() {
                continue;
            }

            // Handle normal text flow
            let glyph = font.get_glyph(char);
            let advance = glyph.advance * font_size;
            let next_width = cursor.x + advance;

            let would_overflow = next_width > container_size.x;

            if would_overflow {
                if self.white_space.wrap() && !overflowed {
                    // Try to split between words
                    if split_point != usize::MAX {
                        let current_line = self.lines.last_mut().unwrap();
                        current_line.end = split_point;

                        let new_line = TextLine {
                            start: split_point,
                            end: current_line.end,
                            width: 0.0,
                        };

                        // remove leading spaces in split line (CSS behavior)
                        if self.white_space.collapses_spaces()
                            && let Some(g) = self.glyphs.last()
                            && g.char.is_whitespace()
                        {
                            self.glyphs.pop();
                        }

                        let mut new_width = 0.0;
                        for g in &mut self.glyphs[new_line.start..new_line.end] {
                            g.pos.x = new_width;
                            g.pos.y += line_height;
                            new_width += g.size.x;
                        }

                        current_line.width -= new_width;

                        self.lines.push(new_line);

                        width = width.max(cursor.x);

                        cursor.x = new_width;
                        split_point = usize::MAX;

                    // Try split in words
                    } else if matches!(self.overflow_wrap, OverflowWrap::BreakWord) {
                        self.lines.push(TextLine {
                            start: self.glyphs.len(),
                            end: self.glyphs.len(),
                            width: 0.0,
                        });

                        width = width.max(cursor.x);

                        cursor.x = 0.0;
                        cursor.y += line_height;
                        split_point = usize::MAX;

                    // Hanlde overflow
                    } else {
                        overflowed = true;
                        match self.overflow {
                            TextOverflow::Allow => (),
                            TextOverflow::Clip => (),
                            TextOverflow::Ellipsis => (),
                        }
                    }
                } else {
                    overflowed = true;
                    match self.overflow {
                        TextOverflow::Allow => (),
                        TextOverflow::Clip => (),
                        TextOverflow::Ellipsis => (),
                    }
                }
            }

            if whitespace || char == '-' {
                split_point = self.glyphs.len();
            }

            let line = self.lines.last_mut().unwrap();

            if !overflowed {
                let right = glyph.right * font_size;
                let left = glyph.left * font_size;
                // this just happend to be the exact number to add to make both '_' and '-' the right size with my testet font_size
                let top = (glyph.top * font_size + 0.4).floor();
                let bottom = (glyph.bottom * font_size).floor();

                
                let size = Vec2::new(right - left, (bottom - top).max(2.0));
                let pos = Vec2::new(left + cursor.x, top + 0.5 + cursor.y.floor());

                self.glyphs.push(Glyph {
                    char,
                    pos,
                    size,
                    uv_start: glyph.atlas_start,
                    uv_end: glyph.atlas_end,
                });

                line.end = self.glyphs.len();
            }

            line.width += advance;
            cursor.x = next_width;
            last_whitespace = whitespace;
        }

        width = width.max(cursor.x);
        self.size = Vec2::new(width, self.lines.len() as f32 * line_height);
    }
}

impl Default for TextLayout {
    fn default() -> Self {
        Self {
            font: None,
            font_size: 16.0,
            line_spacing: 1.0,
            overflow: TextOverflow::default(),
            overflow_wrap: OverflowWrap::default(),
            white_space: WhiteSpace::default(),
            lines: Vec::default(),
            glyphs: Vec::default(),
            size: Vec2::default(),
        }
    }
}

/// Represents a single line of processed text after layout.
#[derive(Default, Debug)]
pub struct TextLine {
    pub start: usize,
    pub end: usize,
    pub width: f32,
}

#[derive(Debug)]
pub struct Glyph {
    pub char: char,
    pub pos: Vec2<f32>,
    pub size: Vec2<f32>,
    pub uv_start: Vec2<f32>,
    pub uv_end: Vec2<f32>,
}
