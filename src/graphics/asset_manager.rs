use std::rc::Rc;

use crate::ui::Font;

pub struct AssetManager {
    _fonts: Rc<[Font]>,
}

impl AssetManager {
    pub fn with_fonts(_fonts: Rc<[Font]>) -> Self {
        Self { _fonts }
    }
}
