mod vertex_layouts;

//pub use single_image::SingleImage;
pub use vertex_layouts::AtlasInstance;
pub use vertex_layouts::FontInstance;
pub use vertex_layouts::ShadowInstance;
pub use vertex_layouts::UiInstance;

#[derive(Debug, Clone, Copy)]
pub enum MatType {
    Basic,
    Font,
    Shadow,
    Atlas,
}
