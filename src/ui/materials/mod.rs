mod vertex_layouts;

pub use vertex_layouts::AtlasInstance;
pub use vertex_layouts::ShadowInstance;
pub use vertex_layouts::UiInstance;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatType {
    Basic,
    Bitmap,
    Shadow,
    Atlas,
    MSDF,
}
