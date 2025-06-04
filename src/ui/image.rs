
use crate::{graphics::formats::Color, primitives::Vec2};

use super::{style::Position, BuildContext, Style, UiElement};



#[derive(Debug, Clone)]
pub struct UiImage {
    pub index: u8,
}

impl UiImage {
    pub fn new(style: Style, index: u8) -> UiElement {
        UiElement::extend(
            style,
            Vec::with_capacity(0),
            super::UiType::Image(Self { index })
        )
    }

    pub fn build(&self, element: &mut UiElement, context: &mut BuildContext) {
        let size;
        let mut pos;

        match &element.style.position {
            Position::Absolute(absolute) => {
                size = Vec2::new(
                    element.style.width.width(context.parent_size),
                    element.style.height.height(context.parent_size)
                );

                pos = absolute.align.get_pos(context.parent_size, size, Vec2::new(absolute.x.pixelx(context.parent_size), absolute.y.pixely(context.parent_size)));

            },
            Position::Inline(_) => {
                size = Vec2::new(
                    element.style.width.width(context.parent_size),
                    element.style.height.height(context.parent_size)
                );

                pos = Vec2::new(0.0, 0.0);

                if context.parent_size.x - context.start_pos.x - context.parent_pos.x >= size.x {
                    pos.x += context.start_pos.x;
                    context.start_pos.x += size.x;
                } else {
                    pos.y += context.start_pos.y;
                    context.start_pos.y += size.y;
                }

            }
        }

        element.computed.border_color = element.style.border_color.as_color();
        element.computed.border = element.style.border[0];
        element.computed.corner = element.style.corner[0].pixelx(size);

        pos += context.parent_pos;

        {
            element.computed.pos = pos;
            element.computed.size = size;
            element.computed.color = Color::new(f32::from_bits(self.index as u32), 0.0, 0.0, 0.0);
            element.computed.mode = 3;
            element.computed.order = context.order;
        }
        element.dirty = false;
    }
}