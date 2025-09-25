use super::{
    BuildContext, ElementType, OutArea, UiElement, UiUnit,
    ui_element::{Element, TypeConst},
};
use crate::{
    graphics::{UiInstance, formats::Color},
    primitives::Vec2,
    ui::{
        FlexDirection,
        draw_data::{DrawData, InstanceData},
    },
};

pub struct Container {
    pub margin: OutArea,
    pub padding: OutArea,
    pub width: UiUnit,
    pub height: UiUnit,
    pub color: Color,
    pub border_color: Color,
    pub flex_direction: FlexDirection,
    pub border: [f32; 4],
    pub corner: [UiUnit; 4],
    pub childs: Vec<UiElement>,
}

impl Element for Container {
    fn build(&mut self, context: &mut BuildContext, element: &UiElement) {
        let space = context.available_size;

        let width = if matches!(self.width, UiUnit::Fill) {
            element.size.x
        } else {
            self.width.pixelx(space)
        };
        let height = if matches!(self.height, UiUnit::Fill) {
            element.size.y
        } else {
            self.height.pixely(space)
        };

        let outer_size = Vec2::new(width, height);

        let size = outer_size - self.margin.size(space);
        let mut pos = self.margin.start(space) + context.child_start_pos;

        context.fits_in_line(&mut pos, outer_size);

        let available_size = size - self.padding.size(space);
        let child_start_pos = pos + self.padding.start(space);

        let mut child_context = BuildContext::new_from(
            context,
            available_size,
            child_start_pos,
            element,
            self.flex_direction,
        );

        for element in self.childs.iter_mut() {
            let (width, height) = element.element.get_size();
            if matches!(width, UiUnit::Fill) {
                element.size.x = (child_context.available_size.x - child_context.start_pos.x).abs();
            }

            if matches!(height, UiUnit::Fill) {
                element.size.y = (child_context.available_size.y - child_context.start_pos.y).abs();
            }
            element.build(&mut child_context);
            child_context.order += 1;
        }

        context.apply_data(pos, size);
    }

    fn get_size(&mut self) -> (UiUnit, UiUnit) {
        (self.width, self.height)
    }

    fn instance(
        &self,
        element: &UiElement,
        draw_data: &mut DrawData,
        clip: Option<ash::vk::Rect2D>,
    ) {
        if let InstanceData::Basic(vec) = draw_data.get_group(0, 0, clip) {
            vec.push(UiInstance {
                color: self.color,
                border_color: self.border_color,
                border: self.border[0],
                x: element.pos.x,
                y: element.pos.y,
                width: element.size.x,
                height: element.size.y,
                corner: self.corner[0].pixelx(element.size),
                z_index: element.z_index,
            });
        } else {
            unreachable!()
        }
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
            color: Color::DARKGREY,
            border_color: Color::GREEN,
            flex_direction: FlexDirection::default(),
            border: [0.0; 4],
            corner: [UiUnit::Zero; 4],
            childs: Default::default(),
        }
    }
}
