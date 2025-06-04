use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
#[cfg(feature = "graphics")]
use cgmath::Vector4;


#[derive(Debug, Clone, Copy, Default)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0, w: 0.0 }
    }

    pub const fn one() -> Self {
        Self { x: 1.0, y: 1.0, z: 1.0, w: 1.0 }
    }

    #[inline(always)]
    pub fn len(&self) -> f32 {
        (self.x.powi(2) + self.y.powi(2) + self.z.powi(2) + self.w.powi(2)).sqrt()
    }

    #[inline(always)]
    pub fn dot(&self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    #[inline(always)]
    pub fn cross(self, other: Vec4) -> Vec4 {
        Vec4::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
            0.0 // Assuming w should be 0.0 for the cross product
        )
    }

    #[inline(always)]
    pub fn distance(&self, other: Self) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2) + (self.w - other.w).powi(2)).sqrt()
    }

    #[inline(always)]
    pub fn normalize(&self) -> Self {
        let len = self.len();
        if len > 0.0 {
            *self / len
        } else {
            *self
        }
    }

    #[inline(always)]
    pub fn magnitude(&self) -> f32 {
        self.len()
    }

    #[inline(always)]
    pub fn lerp(&self, other: Self, t: f32) -> Self {
        *self * (1.0 - t) + other * t
    }
}

impl Add for Vec4 {
    type Output = Vec4;

    fn add(self, other: Vec4) -> Vec4 {
        Vec4 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            w: self.w + other.w,
        }
    }
}

impl Add<f32> for Vec4 {
    type Output = Vec4;

    fn add(self, other: f32) -> Vec4 {
        Vec4 {
            x: self.x + other,
            y: self.y + other,
            z: self.z + other,
            w: self.w + other,
        }
    }
}


impl AddAssign for Vec4 {
    fn add_assign(&mut self, other: Vec4) {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
    }
}

impl Mul for Vec4 {
    type Output = Vec4;

    fn mul(self, other: Vec4) -> Vec4 {
        Vec4 {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
            w: self.w * other.w,
        }
    }
}

impl Mul<f32> for Vec4 {
    type Output = Vec4;

    fn mul(self, other: f32) -> Vec4 {
        Vec4 {
            x: self.x * other,
            y: self.y * other,
            z: self.z * other,
            w: self.w * other,
        }
    }
}


impl MulAssign for Vec4 {
    fn mul_assign(&mut self, other: Vec4) {
        self.x *= other.x;
        self.y *= other.y;
        self.z *= other.z;
        self.w *= other.w;
    }
}

impl Div for Vec4 {
    type Output = Vec4;

    fn div(self, other: Vec4) -> Vec4 {
        Vec4 {
            x: self.x / other.x,
            y: self.y / other.y,
            z: self.z / other.z,
            w: self.w / other.w,
        }
    }
}

impl Div<f32> for Vec4 {
    type Output = Vec4;

    fn div(self, other: f32) -> Vec4 {
        Vec4 {
            x: self.x / other,
            y: self.y / other,
            z: self.z / other,
            w: self.w / other,
        }
    }
}

impl DivAssign for Vec4 {
    fn div_assign(&mut self, other: Vec4) {
        self.x /= other.x;
        self.y /= other.y;
        self.z /= other.z;
        self.w /= other.w;
    }
}

impl DivAssign<f32> for Vec4 {

    fn div_assign(&mut self, other: f32) {
        self.x /= other;
        self.y /= other;
        self.z /= other;
        self.w /= other;
    }
}

impl Sub for Vec4 {
    type Output = Vec4;

    fn sub(self, other: Vec4) -> Vec4 {
        Vec4 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
            w: self.w - other.w,
        }
    }
}

impl Sub<f32> for Vec4 {
    type Output = Vec4;

    fn sub(self, other: f32) -> Vec4 {
        Vec4 {
            x: self.x - other,
            y: self.y - other,
            z: self.z - other,
            w: self.w - other,
        }
    }
}

// Implementiere SubAssign für den `-=` Operator
impl SubAssign for Vec4 {
    fn sub_assign(&mut self, other: Vec4) {
        self.x -= other.x;
        self.y -= other.y;
        self.z -= other.z;
        self.w -= other.w;
    }
}

impl PartialEq for Vec4 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z && self.w == other.w
    }
}

impl PartialOrd for Vec4 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.x > other.x && self.y > other.y {
            Some(std::cmp::Ordering::Greater)
        } else if self.x < other.x && self.y < other.y {
            Some(std::cmp::Ordering::Less)
        } else if self == other {
            Some(std::cmp::Ordering::Equal)
        } else {
            None
        }
    }
}

impl Neg for Vec4 {
    type Output = Vec4;

    fn neg(self) -> Vec4 {
        Vec4::new(-self.x, -self.y, -self.z, -self.w)
    }
}

#[cfg(feature = "graphics")]
impl From<Vector4<f32>> for Vec4 {
    fn from(v: Vector4<f32>) -> Self {
        Self { x: v.x, y: v.y, z: v.z, w: v.w }
    }
}

#[cfg(feature = "graphics")]
impl From<Vec4> for Vector4<f32> {
    fn from(v: Vec4) -> Self {
        Self { x: v.x, y: v.y, z: v.z, w: v.w }
    }
}