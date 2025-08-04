use super::{
    Align, BuildContext, ElementType, OutArea, RawUiElement, UiElement, UiUnit,
    ui_element::{Element, TypeConst},
};
use crate::{graphics::formats::Color, primitives::Vec2};

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
    pub comp: RawUiElement,
    pub childs: Vec<UiElement>,
}

impl Element for AbsoluteLayout {
    fn build(&mut self, context: &mut BuildContext, _: &UiElement) {
        let mut size = Vec2::new(
            self.width.pixelx(context.parent_size),
            self.height.pixely(context.parent_size),
        );

        let mut pos = self.align.get_pos(
            context.parent_size,
            size,
            Vec2::new(
                self.x.pixelx(context.parent_size),
                self.y.pixely(context.parent_size),
            ),
        );

        let comp = &mut self.comp;

        comp.border = self.border[0];
        comp.corner = self.corner[0].pixelx(size);

        pos += context.parent_pos;

        let mut child_context =
            BuildContext::new_from(context, size, pos + self.padding.start(size), &comp);

        size.x += self.padding.x(child_context.parent_size);
        size.y += self.padding.y(child_context.parent_size);

        comp.size = size;
        comp.pos = pos;

        for element in self.childs.iter_mut() {
            element.build(&mut child_context);
            child_context.order += 1;
        }

        context.apply_data(pos, size);
    }

    fn instance(&self, element: &UiElement) -> crate::graphics::UiInstance {
        self.comp.to_instance(self.color, self.border_color, element.z_index)
    }

    fn childs(&mut self) -> &mut [UiElement] {
        &mut self.childs
    }

    fn add_child(&mut self, child: UiElement) {
        self.childs.push(child);
    }
}

impl TypeConst for AbsoluteLayout {
    const ELEMENT_TYPE: ElementType = ElementType::AbsoluteLayout;
}

impl Default for AbsoluteLayout {
    fn default() -> Self {
        Self {
            comp: Default::default(),
            childs: Default::default(),
            align: Align::Center,
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
