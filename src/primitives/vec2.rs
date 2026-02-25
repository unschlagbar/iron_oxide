use std::{
    cmp::Ordering,
    fmt,
    ops::{Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign},
};

#[derive(Debug, Clone, Default)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T,
}

impl<T: Copy> Copy for Vec2<T> {}

impl<T> Vec2<T> {
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T: Add<Output = T> + Copy> Add<Vec2<T>> for Vec2<T> {
    type Output = Vec2<T>;
    fn add(self, other: Vec2<T>) -> Vec2<T> {
        Vec2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl<T: Add<Output = T> + Copy> Add<T> for Vec2<T> {
    type Output = Vec2<T>;
    fn add(self, other: T) -> Vec2<T> {
        Vec2 {
            x: self.x + other,
            y: self.y + other,
        }
    }
}

impl<T: AddAssign + Copy> AddAssign<Vec2<T>> for Vec2<T> {
    fn add_assign(&mut self, other: Vec2<T>) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl<T: Sub<Output = T> + Copy> Sub<Vec2<T>> for Vec2<T> {
    type Output = Vec2<T>;
    fn sub(self, other: Vec2<T>) -> Vec2<T> {
        Vec2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl<T: Sub<Output = T> + Copy> Sub<T> for Vec2<T> {
    type Output = Vec2<T>;
    fn sub(self, other: T) -> Vec2<T> {
        Vec2 {
            x: self.x - other,
            y: self.y - other,
        }
    }
}

impl<T: SubAssign + Copy> SubAssign<Vec2<T>> for Vec2<T> {
    fn sub_assign(&mut self, other: Vec2<T>) {
        self.x -= other.x;
        self.y -= other.y;
    }
}

impl<T: Mul<Output = T> + Copy> Mul<Vec2<T>> for Vec2<T> {
    type Output = Vec2<T>;
    fn mul(self, other: Vec2<T>) -> Vec2<T> {
        Vec2 {
            x: self.x * other.x,
            y: self.y * other.y,
        }
    }
}

impl<T: Mul<Output = T> + Copy> Mul<T> for Vec2<T> {
    type Output = Vec2<T>;
    fn mul(self, other: T) -> Vec2<T> {
        Vec2 {
            x: self.x * other,
            y: self.y * other,
        }
    }
}

impl<T: MulAssign + Copy> MulAssign<Vec2<T>> for Vec2<T> {
    fn mul_assign(&mut self, other: Vec2<T>) {
        self.x *= other.x;
        self.y *= other.y;
    }
}

impl<T: Div<Output = T> + Copy> Div<Vec2<T>> for Vec2<T> {
    type Output = Vec2<T>;
    fn div(self, other: Vec2<T>) -> Vec2<T> {
        Vec2 {
            x: self.x / other.x,
            y: self.y / other.y,
        }
    }
}

impl<T: Div<Output = T> + Copy> Div<T> for Vec2<T> {
    type Output = Vec2<T>;
    fn div(self, other: T) -> Vec2<T> {
        Vec2 {
            x: self.x / other,
            y: self.y / other,
        }
    }
}

impl<T: DivAssign + Copy> DivAssign<Vec2<T>> for Vec2<T> {
    fn div_assign(&mut self, other: Vec2<T>) {
        self.x /= other.x;
        self.y /= other.y;
    }
}

impl<T: DivAssign + Copy> DivAssign<T> for Vec2<T> {
    fn div_assign(&mut self, other: T) {
        self.x /= other;
        self.y /= other;
    }
}

impl<T: PartialEq> PartialEq for Vec2<T> {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl<T: PartialOrd + PartialEq> PartialOrd for Vec2<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.x > other.x && self.y > other.y {
            Some(Ordering::Greater)
        } else if self.x < other.x && self.y < other.y {
            Some(Ordering::Less)
        } else if self == other {
            Some(Ordering::Equal)
        } else {
            None
        }
    }
}

impl<T: Neg<Output = T> + Copy> Neg for Vec2<T> {
    type Output = Vec2<T>;
    fn neg(self) -> Vec2<T> {
        Vec2::new(-self.x, -self.y)
    }
}

impl<T: fmt::Display> fmt::Display for Vec2<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Vec{{x: {}, y: {}}}", self.x, self.y)
    }
}

impl<T> Index<usize> for Vec2<T> {
    type Output = T;
    #[track_caller]
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            _ => panic!("Index out of bounds!"),
        }
    }
}

impl<T> IndexMut<usize> for Vec2<T> {
    #[track_caller]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            _ => panic!("Index out of bounds!"),
        }
    }
}

impl Vec2<f32> {
    pub const MAX: Self = Self {
        x: f32::MAX,
        y: f32::MAX,
    };
    pub const MIN: Self = Self {
        x: f32::MIN,
        y: f32::MIN,
    };
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
    pub const fn one() -> Self {
        Self { x: 1.0, y: 1.0 }
    }
    pub const fn min(&self) -> f32 {
        if self.x < self.y { self.x } else { self.y }
    }
    pub const fn max(&self) -> f32 {
        if self.x > self.y { self.x } else { self.y }
    }
    pub const fn max_other(&self, other: &Self) -> Vec2<f32> {
        Vec2::new(self.x.max(other.x), self.y.max(other.y))
    }
    pub const fn min_other(&self, other: &Self) -> Vec2<f32> {
        Vec2::new(self.x.min(other.x), self.y.min(other.y))
    }
    #[inline(always)]
    pub fn len(&self) -> f32 {
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }
}

#[cfg(feature = "vulkan")]
use ash::vk::{Extent2D, Offset2D};
#[cfg(feature = "vulkan")]
use winit::dpi::{PhysicalPosition, PhysicalSize};
#[cfg(feature = "vulkan")]
impl From<PhysicalSize<u32>> for Vec2<f32> {
    fn from(size: PhysicalSize<u32>) -> Self {
        Vec2::new(size.width as f32, size.height as f32)
    }
}
#[cfg(feature = "vulkan")]
impl From<PhysicalPosition<f64>> for Vec2<f32> {
    fn from(size: PhysicalPosition<f64>) -> Self {
        Vec2::new(size.x as f32, size.y as f32)
    }
}
#[cfg(feature = "vulkan")]
impl From<Vec2<f32>> for Offset2D {
    fn from(size: Vec2<f32>) -> Self {
        Offset2D {
            x: size.x as _,
            y: size.y as _,
        }
    }
}
#[cfg(feature = "vulkan")]
impl From<Vec2<f32>> for Extent2D {
    fn from(size: Vec2<f32>) -> Self {
        Extent2D {
            width: size.x as _,
            height: size.y as _,
        }
    }
}

#[cfg(feature = "vulkan")]
impl From<PhysicalPosition<f64>> for Vec2<i16> {
    fn from(size: PhysicalPosition<f64>) -> Self {
        Vec2::new(size.x as i16, size.y as i16)
    }
}

impl Vec2<i16> {
    pub const MAX: Self = Self {
        x: i16::MAX,
        y: i16::MAX,
    };
    pub const MIN: Self = Self {
        x: i16::MIN,
        y: i16::MIN,
    };
    pub const fn one() -> Self {
        Self { x: 1, y: 1 }
    }
    pub const fn min(&self) -> i16 {
        if self.x < self.y { self.x } else { self.y }
    }
    pub const fn max(&self) -> i16 {
        if self.x > self.y { self.x } else { self.y }
    }
    pub fn into_f32(self) -> Vec2<f32> {
        Vec2 {
            x: self.x as f32,
            y: self.y as f32,
        }
    }
}

#[cfg(feature = "vulkan")]
impl From<Vec2<i16>> for Offset2D {
    fn from(size: Vec2<i16>) -> Self {
        Offset2D {
            x: size.x as _,
            y: size.y as _,
        }
    }
}
#[cfg(feature = "vulkan")]
impl From<Vec2<i16>> for Extent2D {
    fn from(size: Vec2<i16>) -> Self {
        Extent2D {
            width: size.x as _,
            height: size.y as _,
        }
    }
}

#[cfg(feature = "vulkan")]
impl From<PhysicalPosition<i32>> for Vec2<i16> {
    fn from(size: PhysicalPosition<i32>) -> Self {
        Self {
            x: size.x as i16,
            y: size.y as i16,
        }
    }
}
impl<T> From<(T, T)> for Vec2<T> {
    fn from(size: (T, T)) -> Self {
        Self {
            x: size.0,
            y: size.1,
        }
    }
}
