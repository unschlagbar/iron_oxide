
use crate::{graphics::UiInstance, primitives::Vec2, ui::UiSize};

use super::{style::Position, BuildContext, Style, UiElement, UiType};

#[derive(Debug, Clone)]
pub struct Text {
    pub font: u16,
    pub text: String,
    pub comp_text: Vec<UiInstance>,
    pub wrap: WrapMode,
    pub mode: u8,
}

impl Text {
    pub fn new(style: Style, font: u16, text: &str, mode: u8) -> UiElement {
        UiElement::extend(
            style,
            Vec::with_capacity(0),
            UiType::Text(Self::text(font, text.to_string(), mode)) 
        )
    }

    pub fn text(font: u16, text: String, mode: u8) -> Self {
        Self { font, comp_text: Vec::with_capacity(text.len()), text, wrap: WrapMode::Word,  mode }
    }

    pub fn set_text(element: &mut UiElement, text: &str) {
        if let UiType::Text(text_comp) = &mut element.inherit {
            text_comp.text = text.to_owned();
        }
        element.dirty = true;
    }

    #[inline]
    pub fn build_text(&mut self, style: &Style, size: Vec2, pos: Vec2, context: &mut BuildContext) {
        if self.text.is_empty() {
            println!("no text");
            return;
        }
    
        if let Position::Inline(inline) = style.position {
            const BSIZE: f32 = 8.0;
            let mut font_size = style.height.height(context.parent_size);
            let scale_factor = font_size / BSIZE;
            font_size += scale_factor * 2.0;
    
            let mut current_width: f32 = 0.0;
            let mut max_width = match style.width {
                UiSize::Auto => context.parent_size.x - context.start_pos.x,
                _ => context.parent_size.x - inline.margin[0].pixelx(context.parent_size) - inline.margin[2].pixelx(context.parent_size),
            };
    
            let mut lines: Vec<Vec<UiInstance>> = vec![];
            let mut current_line = Vec::new();
    
            let mut x_progress = pos.x + inline.margin[0].pixelx(size);
            let mut y_progress = pos.y + inline.margin[1].pixely(size) + scale_factor;
    
            let mut char_width;
    
            match self.wrap {
                WrapMode::None => {
                    for c in self.text.chars() {
                        if c == '\n' {
                            lines.push(current_line);
                            current_line = Vec::new();
                            current_width = 0.0;
                            y_progress += font_size;
    
                            if y_progress - pos.y > context.line_offset {
                                x_progress -= context.start_pos.x;
                                max_width += context.start_pos.x;
                                context.start_pos.x = 0.0;
                            }
                        } else {
                            let (uv_x, uv_y, char_w, _) = (unsafe { &*context.font }).get_data(c as u8);
                            char_width = char_w as f32 * scale_factor;
    
                            let uv_x_w = f32::from_bits(((char_w as u32) << 16) | (uv_x as u32));
                            let uv_y_h = f32::from_bits(((BSIZE as u32) << 16) | (uv_y as u32));
    
                            current_line.push(UiInstance {
                                color: style.color.as_color(),
                                border_color: style.color.as_color(),
                                border: uv_x_w,
                                x: x_progress + current_width,
                                y: y_progress,
                                width: char_width,
                                height: BSIZE * scale_factor,
                                corner: uv_y_h,
                                mode: 1,
                            });
                            current_width += char_width;
                        }
                    }
                }
                WrapMode::Character => {
                    for c in self.text.chars() {
                        if c == '\n' {
                            lines.push(current_line);
                            current_line = Vec::new();
                            current_width = 0.0;
                            y_progress += font_size;
    
                            if y_progress - pos.y > context.line_offset {
                                x_progress -= context.start_pos.x;
                                max_width += context.start_pos.x;
                                context.start_pos.x = 0.0;
                            }
                        } else {
                            let (uv_x, uv_y, char_w, _) = (unsafe { &*context.font }).get_data(c as u8);
                            char_width = char_w as f32 * scale_factor;
    
                            if current_width + char_width > max_width {
                                lines.push(current_line);
                                current_line = Vec::new();
                                current_width = 0.0;
                                y_progress += font_size;
    
                                if y_progress - pos.y > context.line_offset {
                                    x_progress -= context.start_pos.x;
                                    max_width += context.start_pos.x;
                                    context.start_pos.x = 0.0;
                                }
                            }
    
                            let uv_x_w = f32::from_bits(((char_w as u32) << 16) | (uv_x as u32));
                            let uv_y_h = f32::from_bits(((BSIZE as u32) << 16) | (uv_y as u32));
    
                            current_line.push(UiInstance {
                                color: style.color.as_color(),
                                border_color: style.color.as_color(),
                                border: uv_x_w,
                                x: x_progress + current_width,
                                y: y_progress,
                                width: char_width,
                                height: BSIZE * scale_factor,
                                corner: uv_y_h,
                                mode: 1,
                            });
                            current_width += char_width;
                        }
                    }
                }
                WrapMode::Word => {
                    let mut word = Vec::new();
                    let mut word_width = 0.0;
                
                    for c in self.text.chars() {
                        if c.is_whitespace() {
                            // Verarbeite das aktuelle Wort (falls vorhanden)
                            if !word.is_empty() {
                                if current_width + word_width > max_width {
                                    // Zeile abschließen, falls das Wort nicht mehr passt
                                    lines.push(current_line);
                                    current_line = Vec::new();
                                    current_width = 0.0;
                                    y_progress += font_size;
                
                                    if y_progress - pos.y > context.line_offset {
                                        x_progress -= context.start_pos.x;
                                        max_width += context.start_pos.x;
                                        context.start_pos.x = 0.0;
                                    }
                                }
                
                                // Füge das Wort zur aktuellen Zeile hinzu
                                for &wc in &word {
                                    let (uv_x, uv_y, char_w, _) = (unsafe { &*context.font }).get_data(wc as u8);
                                    let uv_x_w = f32::from_bits(((char_w as u32) << 16) | (uv_x as u32));
                                    let uv_y_h = f32::from_bits(((BSIZE as u32) << 16) | (uv_y as u32));
                                    char_width = char_w as f32 * scale_factor;
                
                                    current_line.push(UiInstance {
                                        color: style.color.as_color(),
                                        border_color: style.color.as_color(),
                                        border: uv_x_w,
                                        x: x_progress + current_width,
                                        y: y_progress,
                                        width: char_width,
                                        height: BSIZE * scale_factor,
                                        corner: uv_y_h,
                                        mode: 1,
                                    });
                
                                    current_width += char_width;
                                }
                
                                word.clear();
                                word_width = 0.0;
                            }
                
                            // Verarbeite das Leerzeichen (falls vorhanden)
                            if c.is_whitespace() {
                                let c = if c == '\n' {' '} else {c};
                                let (uv_x, uv_y, char_w, _) = (unsafe { &*context.font }).get_data(c as u8);
                                char_width = char_w as f32 * scale_factor;
                
                                if current_width + char_width > max_width {
                                    // Zeile abschließen, falls das Leerzeichen nicht mehr passt
                                    lines.push(current_line);
                                    current_line = Vec::new();
                                    current_width = 0.0;
                                    y_progress += font_size;
                
                                    if y_progress - pos.y > context.line_offset {
                                        x_progress -= context.start_pos.x;
                                        max_width += context.start_pos.x;
                                        context.start_pos.x = 0.0;
                                    }
                                }
                
                                let uv_x_w = f32::from_bits(((char_w as u32) << 16) | (uv_x as u32));
                                let uv_y_h = f32::from_bits(((BSIZE as u32) << 16) | (uv_y as u32));
                
                                current_line.push(UiInstance {
                                    color: style.color.as_color(),
                                    border_color: style.color.as_color(),
                                    border: uv_x_w,
                                    x: x_progress + current_width,
                                    y: y_progress,
                                    width: char_width,
                                    height: BSIZE * scale_factor,
                                    corner: uv_y_h,
                                    mode: 1,
                                });
                
                                current_width += char_width;
                            }
                
                            // Bei einem Zeilenumbruch die aktuelle Zeile abschließen
                            if c == '\n' {
                                lines.push(current_line);
                                current_line = Vec::new();
                                current_width = 0.0;
                                y_progress += font_size;
                
                                if y_progress - pos.y > context.line_offset {
                                    x_progress -= context.start_pos.x;
                                    max_width += context.start_pos.x;
                                    context.start_pos.x = 0.0;
                                }
                            }
                        } else {
                            // Buchstabe zum Wort hinzufügen
                            let (_, _, char_w, _) = (unsafe { &*context.font }).get_data(c as u8);
                            let scaled_char_w = char_w as f32 * scale_factor;
                            word.push(c);
                            word_width += scaled_char_w;
                        }
                    }
                
                    // Verarbeite das letzte Wort (falls vorhanden)
                    if !word.is_empty() {
                        if current_width + word_width > max_width {
                            lines.push(current_line);
                            current_line = Vec::new();
                            current_width = 0.0;
                            y_progress += font_size;
                
                            if y_progress - pos.y > context.line_offset {
                                x_progress -= context.start_pos.x;
                                max_width += context.start_pos.x;
                                context.start_pos.x = 0.0;
                            }
                        }
                
                        for &wc in &word {
                            let (uv_x, uv_y, char_w, _) = (unsafe { &*context.font }).get_data(wc as u8);
                            let uv_x_w = f32::from_bits(((char_w as u32) << 16) | (uv_x as u32));
                            let uv_y_h = f32::from_bits(((BSIZE as u32) << 16) | (uv_y as u32));
                            char_width = char_w as f32 * scale_factor;
                
                            current_line.push(UiInstance {
                                color: style.color.as_color(),
                                border_color: style.color.as_color(),
                                border: uv_x_w,
                                x: x_progress + current_width,
                                y: y_progress,
                                width: char_width,
                                height: BSIZE * scale_factor,
                                corner: uv_y_h,
                                mode: 1,
                            });
                
                            current_width += char_width;
                        }
                    }
                }
            }
    
            if !current_line.is_empty() {
                lines.push(current_line);
            }
    
            let y_offset = match self.mode & 0b10 {
                0 => 0.0,
                2 => (context.parent_size.y - font_size) * 0.5,
                _ => 0.0,
            };
    
            let mut final_comp_text = Vec::new();
            for line in &mut lines {
                let line_width: f32 = line.iter().map(|i| i.width).sum();
                let x_offset = match self.mode & 0b1 {
                    0 => 0.0,
                    1 => (max_width - line_width) * 0.5,
                    _ => 0.0,
                };
    
                for instance in line {
                    instance.x += x_offset;
                    instance.y += y_offset;
                    final_comp_text.push(*instance);
                }
            }
    
            self.comp_text = final_comp_text;
            context.start_pos.x = current_width;
            context.start_pos.y += font_size * (lines.len() - 1) as f32;
            context.line_offset = context.line_offset.max(font_size);
        } else {
            unreachable!("Only Inline is allowed for Text!")
        }
    }
    



    pub fn build(&self, element: &mut UiElement, context: &mut BuildContext) {

        let pos;

        match &element.style.position {
            Position::Inline(_) => {
                pos = Vec2::new(
                    context.start_pos.x,
                    context.start_pos.y
                );
                //let style = element.style.clone();

                element.computed.color = element.style.color.as_color();
                element.computed.border_color = element.style.border_color.as_color();
                element.computed.border = element.style.border[0];
            }
            Position::Absolute(_) => {
                panic!();
            },
        }

        element.computed.pos = pos + context.parent_pos;
        element.computed.order = context.order;

        let text = unsafe { (self as *const Text).cast_mut().as_mut().unwrap_unchecked() };
        text.build_text(&element.style, Vec2::zero(), element.computed.pos, context);
        element.computed.size.x = context.start_pos.x - pos.x;
        element.computed.size.y = context.start_pos.y - pos.y + context.line_offset;
        element.dirty = false;
    }
}

#[derive(Debug, Clone)]
pub enum WrapMode {
    Character,
    Word,
    None,
}