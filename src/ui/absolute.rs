use super::{
    Align, BuildContext, ElementType, OutArea, UiElement, UiUnit,
    element::{Element, TypeConst},
};
use crate::{
    graphics::formats::RGBA,
    primitives::Vec2,
    ui::{FlexDirection, UiState, materials::UiInstance},
};

pub struct Absolute {
    pub align: Align,
    pub x: UiUnit,
    pub y: UiUnit,
    pub width: UiUnit,
    pub height: UiUnit,
    pub color: RGBA,
    pub border_color: RGBA,
    pub border: [u8; 4],
    pub corner: [UiUnit; 4],
    pub padding: OutArea,
    pub childs: Vec<UiElement>,
}

impl Element for Absolute {
    fn build(&mut self, context: &mut BuildContext) {
        let space = context.available_size;
        let padding = self.padding.size(context);

        let width = self.width.pixelx(context);
        let height = self.height.pixely(context);

        let mut size = Vec2::new(width, height);

        let pos = context.child_start_pos
            + self.align.get_pos(
                space,
                size,
                Vec2::new(self.x.pixelx(context), self.y.pixely(context)),
            );

        let mut child_ctx = BuildContext::new_from(
            context,
            size - padding,
            pos + self.padding.start(context),
            FlexDirection::default(),
        );

        for c in &mut self.childs {
            c.build(&mut child_ctx);
        }

        // use autosize if width or height was auto
        if matches!(self.width, UiUnit::Auto) {
            size.x = child_ctx.final_size().x + padding.x;
        }

        if matches!(self.height, UiUnit::Auto) {
            size.y = child_ctx.final_size().y + padding.y;
        }

        context.apply_data(pos, size);
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
            corner: self.corner[0].px(element.size),
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

impl TypeConst for Absolute {
    const ELEMENT_TYPE: ElementType = ElementType::Absolute;
}

impl Default for Absolute {
    fn default() -> Self {
        Self {
            childs: Default::default(),
            align: Align::default(),
            x: UiUnit::Px(10.0),
            y: UiUnit::Px(10.0),
            width: UiUnit::Px(100.0),
            height: UiUnit::Px(100.0),
            color: RGBA::DARKGREY,
            border_color: RGBA::GREEN,
            border: [0; 4],
            corner: [UiUnit::Zero; 4],
            padding: OutArea::default(),
        }
    }
}
