#![allow(dead_code)]

use std::{
    alloc::{alloc, dealloc, Layout},
    ops::{Index, IndexMut},
    ptr::NonNull,
};

pub struct Vec3D {
    data: NonNull<f32>, // Rohspeicher für die Matrixdaten
    depth: usize,
    rows: usize,
    cols: usize,
}

impl Vec3D {
    pub fn new(depth: usize, rows: usize, cols: usize) -> Self {
        let size = depth * rows * cols;
        let layout = unsafe { Layout::array::<f32>(size).unwrap_unchecked() };

        let data = unsafe {
            let ptr = alloc(layout) as *mut f32;

            #[cfg(debug_assertions)]
            if ptr.is_null() {
                panic!("Speicherallokation fehlgeschlagen");
            }
            NonNull::new_unchecked(ptr)
        };

        Vec3D { data, depth, rows, cols }
    }

    pub const fn depth(&self) -> usize {
        self.depth
    }

    pub const fn rows(&self) -> usize {
        self.rows
    }

    pub const fn cols(&self) -> usize {
        self.cols
    }

    pub fn set(&mut self, depth: usize, row: usize, col: usize, value: f32) {
        debug_assert!(depth < self.depth && row < self.rows && col < self.cols);

        let index = depth * self.rows * self.cols + row * self.cols + col;
        unsafe { *self.data.as_ptr().add(index) = value };
    }

    pub fn get(&self, depth: usize, row: usize, col: usize) -> f32 {
        debug_assert!(depth < self.depth && row < self.rows && col < self.cols);

        let index = depth * self.rows * self.cols + row * self.cols + col;
        unsafe { *self.data.as_ptr().add(index) }
    }
}

impl Index<(usize, usize, usize)> for Vec3D {
    type Output = f32;

    fn index(&self, index: (usize, usize, usize)) -> &Self::Output {
        let (depth, row, col) = index;
        debug_assert!(depth < self.depth && row < self.rows && col < self.cols);

        let idx = depth * self.rows * self.cols + row * self.cols + col;
        unsafe { &*self.data.as_ptr().add(idx) }
    }
}

// Implementierung von IndexMut, um das Schreiben über [] zu ermöglichen
impl IndexMut<(usize, usize, usize)> for Vec3D {
    fn index_mut(&mut self, index: (usize, usize, usize)) -> &mut Self::Output {
        let (depth, row, col) = index;
        debug_assert!(depth < self.depth && row < self.rows && col < self.cols);

        let idx = depth * self.rows * self.cols + row * self.cols + col;
        unsafe { &mut *self.data.as_ptr().add(idx) }
    }
}

impl Drop for Vec3D {
    fn drop(&mut self) {
        let size = self.depth * self.rows * self.cols;
        let layout = unsafe { Layout::array::<f32>(size).unwrap_unchecked() };
        unsafe {
            dealloc(self.data.as_ptr() as _, layout);
        }
    }
}
