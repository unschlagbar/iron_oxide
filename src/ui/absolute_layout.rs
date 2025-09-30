use super::{
    Align, BuildContext, ElementType, OutArea, UiElement, UiUnit,
    element::{Element, TypeConst},
};
use crate::{
    graphics::{UiInstance, formats::Color},
    primitives::Vec2,
    ui::{FlexDirection, UiState},
};

pub struct AbsoluteLayout {
    pub align: Align,
    pub x: UiUnit,
    pub y: UiUnit,
    pub width: UiUnit,
    pub height: UiUnit,
    pub color: Color,
    pub border_color: Color,
    pub border: [f32; 4],
    pub corner: [UiUnit; 4],
    pub padding: OutArea,
    pub childs: Vec<UiElement>,
}

impl Element for AbsoluteLayout {
    fn build(&mut self, context: &mut BuildContext, element: &UiElement) {
        let space = context.available_size;

        let mut rework_width = false;
        let mut rework_height = false;

        let width = if self.width == UiUnit::Auto {
            rework_width = true;
            0.0
        } else {
            self.width.pixelx(space)
        };
        let height = if self.height == UiUnit::Auto {
            rework_height = true;
            0.0
        } else {
            self.height.pixely(space)
        };
        let mut size = Vec2::new(width, height);

        let pos = context.child_start_pos
            + self.align.get_pos(
                space,
                size,
                Vec2::new(self.x.pixelx(space), self.y.pixely(space)),
            );

        let mut child_context = BuildContext::new_from(
            context,
            size,
            pos + self.padding.start(size),
            element,
            FlexDirection::Vertical,
        );

        size.x += self.padding.x(child_context.available_size);
        size.y += self.padding.y(child_context.available_size);

        for element in &mut self.childs {
            element.build(&mut child_context);
            child_context.order += 1;
        }

        if rework_width {
            size.x = child_context.start_pos.x;
        }

        if rework_height {
            size.y = child_context.start_pos.y;
        }

        println!("pos: {:?}, size: {:?}", pos, size);

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
            border: self.border[0],
            x: element.pos.x,
            y: element.pos.y,
            width: element.size.x,
            height: element.size.y,
            corner: self.corner[0].pixelx(element.size),
            z_index: element.z_index,
        };
        material.add(&to_add as *const _ as *const _, 0, clip);
    }

    fn childs_mut(&mut self) -> Option<&mut Vec<UiElement>> {
        Some(&mut self.childs)
    }

    fn childs(&self) -> &[UiElement] {
        &self.childs
    }
}

impl TypeConst for AbsoluteLayout {
    const ELEMENT_TYPE: ElementType = ElementType::AbsoluteLayout;
}

impl Default for AbsoluteLayout {
    fn default() -> Self {
        Self {
            childs: Default::default(),
            align: Align::TopLeft,
            x: UiUnit::Px(10.0),
            y: UiUnit::Px(10.0),
            width: UiUnit::Px(100.0),
            height: UiUnit::Px(100.0),
            color: Color::DARKGREY,
            border_color: Color::GREEN,
            border: [0.0; 4],
            corner: [UiUnit::Zero; 4],
            padding: OutArea::default(),
        }
    }
}
