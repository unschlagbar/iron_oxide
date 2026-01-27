use crate::{primitives::Vec2, ui::BuildContext};

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
    pub font_size: f32,
    pub line_spacing: f32,
    pub overflow: TextOverflow,
    pub overflow_wrap: OverflowWrap,
    pub white_space: WhiteSpace,
}

impl TextLayout {
    pub fn build(&self, text: &str, context: &mut BuildContext) -> LayoutText {
        let container_size = context.available_size;

        let font = context.font();
        let uv_height = font.height;
        let font_size = self.font_size * context.scale_factor;
        let mut layout = LayoutText::new(uv_height, font_size);

        let mut current_width = 0.0;
        let mut last_whitespace = true;
        let mut last_splitable = false;
        let mut split_point = i32::MAX;

        let line_height = font_size + font_size / 8.0 * self.line_spacing;
        let uv_scale = font_size / uv_height as f32;

        for mut c in text.chars() {
            let whitespace = c.is_whitespace();
            let mut overflowed = false;

            if c == '\n' {
                if self.white_space.newlines() {
                    layout.lines.push(TextLine::default());

                    layout.size.y += line_height;
                    layout.size.x = layout.size.x.max(current_width);

                    current_width = 0.0;
                    split_point = i32::MAX;
                    continue;
                } else {
                    c = ' '
                }
            }

            if last_splitable {
                split_point = 0;
            } else {
                split_point = split_point.saturating_add(1);
            }

            // Handle space collapsing
            if whitespace && last_whitespace && self.white_space.collapses_spaces() {
                continue;
            }

            // Handle normal text flow
            let char_width = font.get_width(c) as f32 * uv_scale;
            let next_width = current_width + char_width;

            let would_overflow = next_width > container_size.x;

            if would_overflow {
                if self.white_space.wrap() && !overflowed {
                    // Try to split between words
                    if split_point != i32::MAX {
                        let current_line = layout.last();
                        let at = current_line.content.len() - split_point as usize;

                        let mut new_line = current_line.content.split_off(at);

                        // remove leading spaces in split line (CSS behavior)
                        if self.white_space.collapses_spaces()
                            && let Some(g) = current_line.content.last()
                            && g.char.is_whitespace()
                        {
                            current_line.content.pop();
                        }

                        let mut new_width = 0.0;
                        for g in &mut new_line {
                            g.pos.x = new_width;
                            g.pos.y += line_height;
                            new_width += g.size.x;
                        }

                        current_line.width -= new_width;

                        layout.lines.push(TextLine {
                            content: new_line,
                            width: new_width,
                        });

                        layout.size.y += line_height;
                        layout.size.x = layout.size.x.max(current_width);

                        current_width = new_width;
                        split_point = i32::MAX;

                    // Try split in words
                    } else if matches!(self.overflow_wrap, OverflowWrap::BreakWord) {
                        layout.lines.push(TextLine::default());

                        layout.size.y += line_height;
                        layout.size.x = layout.size.x.max(current_width);

                        current_width = 0.0;
                        split_point = i32::MAX;

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

            if !overflowed {
                let y = layout.size.y - font_size;
                let line = layout.last();
                let pos = Vec2::new(line.width, y);
                let uv = font.get_uv(c);

                line.content.push(Glyph {
                    char: c,
                    pos,
                    size: Vec2::new(char_width, font_size),
                    uv_start: (uv.0, uv.1),
                    uv_size: (uv.2, uv_height),
                });
            }

            layout.last().width += char_width;
            current_width += char_width;
            last_whitespace = whitespace;
            last_splitable = last_whitespace || c == '-'
        }

        layout.size.x = layout.size.x.max(current_width);

        layout
    }
}

impl Default for TextLayout {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            line_spacing: 1.0,
            overflow: TextOverflow::default(),
            overflow_wrap: OverflowWrap::default(),
            white_space: WhiteSpace::default(),
        }
    }
}

/// Represents a single line of processed text after layout.
#[derive(Default, Debug)]
pub struct TextLine {
    pub content: Vec<Glyph>,
    pub width: f32,
}

/// Represents the result of text layout before rendering.
#[derive(Debug)]
pub struct LayoutText {
    pub lines: Vec<TextLine>,
    pub size: Vec2<f32>,
    pub uv_height: u16,
}

impl LayoutText {
    fn new(uv_height: u16, font_size: f32) -> Self {
        Self {
            lines: vec![TextLine::default()],
            size: Vec2::new(0.0, font_size),
            uv_height,
        }
    }

    fn last(&mut self) -> &mut TextLine {
        self.lines.last_mut().unwrap()
    }
}

#[derive(Debug)]
pub struct Glyph {
    pub char: char,
    pub pos: Vec2<f32>,
    pub size: Vec2<f32>,
    pub uv_start: (u16, u16),
    pub uv_size: (u16, u16),
}
