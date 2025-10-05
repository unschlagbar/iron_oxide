#[derive(Debug, Clone, Copy)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RGB {
    pub const ZERO: RGB = RGB { r: 0, g: 0, b: 0 };
    pub const BLACK: RGB = RGB { r: 0, g: 0, b: 0 };
    pub const GREY: RGB = RGB {
        r: 10,
        g: 100,
        b: 100,
    };
    pub const WHITE: RGB = RGB {
        r: 255,
        g: 255,
        b: 255,
    };
    pub const GREEN: RGB = RGB {
        r: 0,
        g: 120,
        b: 20,
    };
    pub const BLUE: RGB = RGB {
        r: 0,
        g: 10,
        b: 150,
    };
    pub const RED: RGB = RGB { r: 255, g: 0, b: 0 };
    pub const PURPLE: RGB = RGB {
        r: 255,
        g: 0,
        b: 255,
    };
    pub const PINK: RGB = RGB {
        r: 255,
        g: 150,
        b: 150,
    };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        RGB { r, g, b }
    }

    pub fn as_color(&self) -> Color {
        Color {
            r: self.r as f32 / 255.0,
            g: self.g as f32 / 255.0,
            b: self.b as f32 / 255.0,
            a: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RGBA {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RGBA {
    pub const ZERO: RGBA = RGBA {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    pub const BLACK: RGBA = RGBA {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const GREY: RGBA = RGBA {
        r: 80,
        g: 80,
        b: 80,
        a: 255,
    };
    pub const WHITE: RGBA = RGBA {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const GREEN: RGBA = RGBA {
        r: 0,
        g: 120,
        b: 20,
        a: 255,
    };
    pub const BLUE: RGBA = RGBA {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };
    pub const RED: RGBA = RGBA {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const PURPLE: RGBA = RGBA {
        r: 255,
        g: 0,
        b: 255,
        a: 255,
    };
    pub const PINK: RGBA = RGBA {
        r: 255,
        g: 150,
        b: 150,
        a: 255,
    };
    pub const DARKGREY: RGBA = RGBA {
        r: 8,
        g: 8,
        b: 8,
        a: 255,
    };
    pub const LIGHTGREY: RGBA = RGBA {
        r: 175,
        g: 175,
        b: 175,
        a: 255,
    };
    pub const TRANSPARENT: RGBA = RGBA {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        RGBA { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        RGBA { r, g, b, a: 255 }
    }

    pub fn as_color(&self) -> Color {
        Color {
            r: self.r as f32 / 255.0,
            g: self.g as f32 / 255.0,
            b: self.b as f32 / 255.0,
            a: self.a as f32 / 255.0,
        }
    }

    pub fn as_u32(&self) -> u32 {
        u32::from_le_bytes([self.r, self.g, self.b, self.a])
    }

    pub const fn to_rgb(&self) -> RGB {
        RGB {
            r: self.r,
            g: self.g,
            b: self.b,
        }
    }
}

impl Default for RGBA {
    fn default() -> Self {
        Self::DARKGREY
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const ZERO: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREY: Self = Self {
        r: 0.05,
        g: 0.4,
        b: 0.4,
        a: 1.0,
    };
    pub const DARKGREY: Self = Self {
        r: 0.05,
        g: 0.05,
        b: 0.05,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 0.5,
        b: 0.03,
        a: 1.0,
    };
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const PURPLE: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const PINK: Self = Self {
        r: 1.0,
        g: 0.6,
        b: 0.6,
        a: 1.0,
    };

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }
}
