use super::{
    BuildContext, ElementType, OutArea, UiElement, UiUnit,
    element::{Element, TypeConst},
};
use crate::{
    graphics::formats::RGBA,
    primitives::Vec2,
    ui::{FlexDirection, UiState, materials::UiInstance},
};

pub struct Container {
    pub margin: OutArea,
    pub padding: OutArea,
    pub width: UiUnit,
    pub height: UiUnit,
    pub color: RGBA,
    pub border_color: RGBA,
    pub flex_direction: FlexDirection,
    pub border: [i8; 4],
    pub corner: [UiUnit; 4],
    pub childs: Vec<UiElement>,
}

impl Element for Container {
    fn build(&mut self, context: &mut BuildContext) {
        // compute outer size
        let margin = self.margin.size(context.available_size);
        let padding = self.padding.size(context.available_size);

        // determine explicit width/height or auto
        let mut final_w = match self.width {
            UiUnit::Fill => context.available_size.x - margin.x,
            _ => self.width.pixelx(context.available_size),
        };

        let mut final_h = match self.height {
            UiUnit::Fill => context.available_size.y - margin.y,
            _ => self.height.pixely(context.available_size),
        };

        let pos = context.pos_child() + self.margin.start(context.available_size);

        let child_start = pos + self.padding.start(context.available_size);

        let mut child_ctx = BuildContext::new_from(
            context,
            Vec2::new(final_w, final_h) - self.padding.size(context.available_size),
            child_start,
            self.flex_direction,
        );

        for c in &mut self.childs {
            let (cw, ch) = c.element.get_size();

            if matches!(cw, UiUnit::Fill) {
                c.size.x = child_ctx.available_size.x;
            }

            if matches!(ch, UiUnit::Fill) {
                c.size.y = child_ctx.available_size.y;
            }

            c.build(&mut child_ctx);
        }

        // use autosize if width or height was auto
        if matches!(self.width, UiUnit::Auto) {
            final_w = child_ctx.final_size().x + padding.x;
        }

        if matches!(self.height, UiUnit::Auto) {
            final_h = child_ctx.final_size().y + padding.y;
        }

        let final_size = Vec2::new(final_w, final_h);

        context.place_child(final_size + margin);
        context.apply_data(pos, final_size);
    }

    fn get_size(&mut self) -> (UiUnit, UiUnit) {
        (self.width, self.height)
    }

    fn instance(&self, element: &UiElement, ui: &mut UiState, clip: Option<ash::vk::Rect2D>) {
        let material = &mut ui.materials[0];
        let to_add = UiInstance {
            color: self.color,
            border_color: self.border_color,
            border: self.border,
            x: element.pos.x as _,
            y: element.pos.y as _,
            width: element.size.x as _,
            height: element.size.y as _,
            corner: self.corner[0].pixelx(element.size),
            z_index: element.z_index,
        };
        material.add(&to_add, 0, clip);
    }

    fn childs_mut(&mut self) -> Option<&mut Vec<UiElement>> {
        Some(&mut self.childs)
    }

    fn childs(&self) -> &[UiElement] {
        &self.childs
    }
}

impl TypeConst for Container {
    const ELEMENT_TYPE: ElementType = ElementType::Block;
}

impl Default for Container {
    fn default() -> Self {
        Self {
            margin: OutArea::default(),
            padding: OutArea::default(),
            width: UiUnit::Px(100.0),
            height: UiUnit::Px(100.0),
            color: RGBA::DARKGREY,
            border_color: RGBA::GREEN,
            flex_direction: FlexDirection::default(),
            border: [0; 4],
            corner: [UiUnit::Zero; 4],
            childs: Default::default(),
        }
    }
}
