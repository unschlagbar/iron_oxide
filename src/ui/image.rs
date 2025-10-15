use crate::ui::element::Element;

pub struct Image {
    pub descriptor: u32,
}

impl Element for Image {
    #[allow(unused)]
    fn build(&mut self, context: &mut super::BuildContext, element: &super::UiElement) {
        todo!()
    }
}
