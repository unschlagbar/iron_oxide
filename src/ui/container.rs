use super::{
    BuildContext, ElementType, OutArea, Overflow, RawUiElement, UiElement, UiUnit,
    ui_element::{Element, ElementBuild, TypeConst},
};
use crate::{graphics::formats::Color, primitives::Vec2};

pub struct Container {
    pub margin: OutArea,
    pub padding: OutArea,
    pub overflow: Overflow,
    pub width: UiUnit,
    pub height: UiUnit,
    pub color: Color,
    pub border_color: Color,
    pub border: [f32; 4],
    pub corner: [UiUnit; 4],
    pub comp: RawUiElement,
    pub childs: Vec<UiElement>,
}

impl Element for Container {
    fn build(&mut self, context: &mut BuildContext) {
        let mut size;
        let mut pos;

        let space = Vec2::new(
            context.parent_size.x - self.padding.x(context.parent_size),
            context.parent_size.y - self.padding.y(context.parent_size),
        );

        size = Vec2::new(self.width.pixelx(space), self.height.pixely(space));

        let mut outer_size = size
            + Vec2::new(
                self.margin.x(context.parent_size),
                self.margin.y(context.parent_size),
            );

        pos = self.margin.start(context.parent_size);

        context.fits_in_line(&mut pos, &mut outer_size);

        let comp = &mut self.comp;

        comp.border = self.border[0];
        comp.corner = self.corner[0].pixelx(size);

        pos += context.parent_pos;

        comp.size = size;
        comp.pos = pos;

        let mut child_context = BuildContext::new_from(context, size, pos, &comp);

        for element in self.childs.iter_mut() {
            element.build(&mut child_context);
            child_context.order += 1;
        }

        if self.width == UiUnit::Auto && child_context.start_pos.x != 0.0 {
            size.x = child_context.start_pos.x
        }
        if self.height == UiUnit::Auto && child_context.start_pos.y != 0.0 {
            comp.size.y = child_context.start_pos.y + child_context.line_offset
        }

        context.apply_data(pos, size);
    }

    fn instance(&self) -> crate::graphics::UiInstance {
        self.comp.to_instance(self.color, self.border_color)
    }

    fn childs(&mut self) -> &mut [UiElement] {
        &mut self.childs
    }

    fn add_child(&mut self, child: UiElement) {
        self.childs.push(child);
    }
}

impl ElementBuild for Container {
    fn wrap(self, ui_state: &super::UiState) -> UiElement {
        UiElement {
            id: ui_state.get_id(),
            typ: ElementType::Block,
            dirty: true,
            visible: true,
            size: Vec2::new(0.0, 0.0),
            pos: Vec2::new(0.0, 0.0),
            parent: std::ptr::null_mut(),
            element: Box::new(self),
        }
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
            overflow: Overflow::hidden(),
            width: UiUnit::Px(100.0),
            height: UiUnit::Px(100.0),
            color: Color::DARKGREY,
            border_color: Color::GREEN,
            border: [0.0; 4],
            corner: [UiUnit::Zero; 4],
            comp: Default::default(),
            childs: Default::default(),
        }
    }
}
