use std::{ops::Range, rc::Rc};

use crate::ui::Align;
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
    /// Bookkeeping, written by [`TextLayout::build`] and [`TextLayout::place`]
    /// and not meant to be set from outside.
    ///
    /// What the glyphs standing in `glyphs` were built from: the width they
    /// were wrapped to and the byte length of the text. A layout pass asks
    /// every element to lay itself out again, and for almost all of them
    /// nothing has changed — [`TextLayout::is_current`] is what makes that
    /// question free instead of a full rebuild.
    ///
    /// NaN until the first build, which is what makes that first one happen.
    pub built_width: f32,
    pub built_len: usize,
    /// Where the glyphs were last moved to, and the space that placed them.
    /// Glyph positions are absolute — drawing, hit testing and the selection
    /// rects all read them straight — so an element that moves has to move its
    /// glyphs with it. Keeping what was applied is what lets that be a
    /// difference instead of a second full offset on top of the first.
    pub applied: Option<(Vec2<f32>, Vec2<f32>)>,
}

impl TextLayout {
    /// Whether the glyphs standing are the ones `build` would produce. The
    /// text is compared by length, not content: every path that edits a text
    /// marks its widget dirty, and this is the second half of that check.
    pub fn is_current(&self, text: &str, ctx: &BuildContext) -> bool {
        self.built_width == ctx.available_space.x && self.built_len == text.len()
    }

    pub fn build(&mut self, text: &str, ctx: &mut BuildContext) {
        let container_size = ctx.available_space;
        self.built_width = container_size.x;
        self.built_len = text.len();
        // Fresh glyphs are positioned relative to the run itself.
        self.applied = None;

        self.glyphs.clear();
        self.glyphs.reserve(text.len());
        self.lines.clear();

        let font = if let Some(font) = &self.font {
            font
        } else {
            ctx.font
        };
        let font_size = self.font_size * ctx.scale_factor;
        let line_height = font.line_height * self.line_spacing * font_size;

        let mut width: f32 = 0.0;

        let mut cursor = Vec2::new(0.0, -font.ascender * font_size);
        let mut last_whitespace = true;
        let mut split_point = usize::MAX;
        // Where the pen stood after the break character, so a wrapped run can be
        // moved without reconstructing it from glyph bounds.
        let mut split_x = 0.0;
        let mut line = self.lines.push_mut(TextLine::default());

        for (index, mut char) in text.chars().enumerate() {
            let whitespace = char.is_whitespace();
            let mut overflowed = false;

            if char == '\n' {
                if self.white_space.newlines() {
                    line = self.lines.push_mut(TextLine {
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
            let mut next_width = cursor.x + advance;

            let would_overflow = next_width > container_size.x;

            if would_overflow {
                if self.white_space.wrap() && !overflowed {
                    // Try to split between words
                    if split_point != usize::MAX {
                        // remove leading spaces in split line (CSS behavior)
                        if self.white_space.collapses_spaces()
                            && let Some(g) = self.glyphs.last()
                            && g.char.is_whitespace()
                        {
                            self.glyphs.pop();
                        }

                        // The space or hyphen the break was taken at closes the
                        // line it sits on; what moves down starts after it.
                        let run_start = (split_point + 1).min(self.glyphs.len());
                        line.end = run_start;

                        // Both the glyphs and the pen move by the pen position the
                        // run began at. Measuring that on ink instead leaves the
                        // two in different frames, which is an overlap of one glyph
                        // right after every break.
                        let moved = cursor.x - split_x;

                        for g in &mut self.glyphs[run_start..] {
                            g.pos.x -= split_x;
                            g.pos.y += line_height;
                        }

                        line.width -= moved;

                        line = self.lines.push_mut(TextLine {
                            start: run_start,
                            end: self.glyphs.len(),
                            width: moved,
                        });

                        width = width.max(split_x);

                        next_width = moved + advance;
                        cursor.x = moved;
                        cursor.y += line_height;
                        split_point = usize::MAX;

                    // Try split in words
                    } else if matches!(self.overflow_wrap, OverflowWrap::BreakWord) {
                        line = self.lines.push_mut(TextLine {
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
                    }
                } else {
                    overflowed = true;
                }
            }

            if whitespace || char == '-' {
                split_point = self.glyphs.len();
                split_x = next_width;
            }

            if overflowed {
                match self.overflow {
                    TextOverflow::Allow => (),
                    TextOverflow::Clip => (),
                    TextOverflow::Ellipsis => (),
                }
            } else {
                let right = glyph.right * font_size;
                let left = glyph.left * font_size;
                // this just happend to be the exact number to add to make both '_' and '-' the right size with my tested font_sizes
                let top = (glyph.top * font_size + 0.4).floor();
                let bottom = (glyph.bottom * font_size).floor();

                let size = Vec2::new(right - left, (bottom - top).max(2.0));
                let pos = Vec2::new(left + cursor.x, top + cursor.y.floor() + 0.5);

                self.glyphs.push(Glyph {
                    char,
                    index: index as u32,
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

    /// Moves the laid-out glyphs so the run sits at `offset` inside `space`.
    /// Safe to call on a layout that was not rebuilt: what was applied last
    /// time is subtracted, so the glyphs land in the same place a rebuild would
    /// have put them.
    pub fn place(&mut self, align: Align, space: Vec2<f32>, offset: Vec2<f32>) {
        let previous = self.applied;
        self.applied = Some((offset, space));

        for line in &self.lines {
            let new_x = align.get_x(space.x, line.width, offset.x);
            let (dx, dy) = match previous {
                Some((old_offset, old_space)) => (
                    new_x - align.get_x(old_space.x, line.width, old_offset.x),
                    offset.y - old_offset.y,
                ),
                None => (new_x, offset.y),
            };
            if dx == 0.0 && dy == 0.0 {
                continue;
            }
            for glyph in &mut self.glyphs[line.range()] {
                glyph.pos.x += dx;
                glyph.pos.y += dy;
            }
        }
    }

    /// Moves the glyphs *without* a layout pass — what scrolling does, through
    /// [`UiElement::offset_element`](crate::ui::UiElement::offset_element).
    ///
    /// The record of where they were placed moves with them, which is what
    /// makes this exact rather than merely close: the next layout pass asks for
    /// the same position it now holds, computes a difference of zero from the
    /// same inputs, and writes nothing at all. A scrolled field therefore
    /// carries exactly one addition per scroll — no rebuild, and no error that
    /// could accumulate over a long scroll.
    pub fn shift(&mut self, by: Vec2<f32>) {
        let Some((offset, _)) = &mut self.applied else {
            // Never placed: there is no absolute position to move yet, and the
            // pass that places it will use the position it finds then.
            return;
        };
        *offset += by;
        for glyph in &mut self.glyphs {
            glyph.pos += by;
        }
    }

    pub fn font<'a>(&'a self, font: &'a Font) -> &'a Font {
        self.font.as_ref().map_or(font, |f| f)
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
            built_width: f32::NAN,
            built_len: 0,
            applied: None,
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

impl TextLine {
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }
}

#[derive(Debug)]
pub struct Glyph {
    pub char: char,
    /// Which character of the source string this is. Not the same as the
    /// glyph's own index: a line break and a collapsed space lay out without
    /// producing a glyph, so anything that colours or marks a *range of the
    /// text* has to count in this rather than in glyphs.
    ///
    /// 32 bits: a `Glyph` is the memory a laid-out field costs — one per
    /// character — and no field holds four billion of them.
    pub index: u32,
    pub pos: Vec2<f32>,
    pub size: Vec2<f32>,
    pub uv_start: Vec2<f32>,
    pub uv_end: Vec2<f32>,
}
