#![allow(dead_code)]

use std::{alloc::{alloc, dealloc, Layout}, fmt::{Debug, Formatter}, mem::forget, ops::{Index, IndexMut, Mul}, ptr::NonNull};

pub struct Matrix {
    data: NonNull<f32>, // Rohspeicher für die Matrixdaten
    rows: usize,
    cols: usize,
}

impl Matrix {
    pub fn new(rows: usize, cols: usize) -> Self {
        let size = rows * cols;
        let layout = unsafe { Layout::array::<f32>(size).unwrap_unchecked() };

        let data = unsafe {
            let ptr = alloc(layout) as *mut f32;

            //#[cfg(debug_assertions)]
            if ptr.is_null() {
                panic!("Speicherallokation fehlgeschlagen");
            }
            NonNull::new_unchecked(ptr)
        };

        Matrix { data, rows, cols }
    }

    pub fn zeroed(rows: usize, cols: usize) -> Self {
        let size = rows * cols;
        let layout = unsafe { Layout::array::<f32>(size).unwrap_unchecked() };

        let data = unsafe {
            let ptr = alloc(layout) as *mut f32;

            //#[cfg(debug_assertions)]
            if ptr.is_null() {
                panic!("Speicherallokation fehlgeschlagen");
            }
            
            ptr.write_bytes(0, size);
            NonNull::new_unchecked(ptr)
        };

        Matrix { data, rows, cols }
    }

    pub const fn rows(&self) -> usize {
        self.rows
    }

    pub const fn cols(&self) -> usize {
        self.cols
    }

    pub const fn flat_len(&self) -> usize {
        self.rows * self.cols
    }

    pub fn set(&mut self, row: usize, col: usize, value: f32) {
        debug_assert!(row < self.rows && col < self.cols);

        let index = row * self.cols + col;
        unsafe { *self.data.as_ptr().add(index) = value };
    }

    pub fn get(&self, row: usize, col: usize) -> f32 {
        debug_assert!(row < self.rows && col < self.cols);

        let index = row * self.cols + col;
        unsafe { *self.data.as_ptr().add(index) }
    }

    pub fn from_slice(slice: &[f32], rows: usize, cols: usize) -> Self {
        debug_assert_eq!(slice.len(), rows * cols, "Slice-Länge stimmt nicht mit den Matrix-Dimensionen überein");
        let matrix = Matrix::new(rows, cols);

        unsafe {
            slice.as_ptr().copy_to_nonoverlapping(matrix.data.as_ptr(), slice.len());
        }

        matrix
    }

    /// Erstellt eine Matrix aus einem Vec
    pub fn from_vec(vec: Vec<f32>, rows: usize, cols: usize) -> Self {
        debug_assert_eq!(vec.len(), rows * cols, "Vec-Länge stimmt nicht mit den Matrix-Dimensionen überein");
        let matrix = Matrix::new(rows, cols);

        // Speicher direkt kopieren
        unsafe {
            vec.as_ptr().copy_to_nonoverlapping(matrix.data.as_ptr(), vec.len());
        }

        // Vec darf hiernach nicht mehr verwendet werden
        std::mem::forget(vec);

        matrix
    }

    pub fn from_vec_no_copy(mut vec: Vec<f32>, rows: usize, cols: usize) -> Self {
        debug_assert_eq!(vec.len(), rows * cols, "Vec-Länge stimmt nicht mit den Matrix-Dimensionen überein");

        // Zeiger aus dem Vec extrahieren
        let ptr = vec.as_mut_ptr();

        // Speicher darf nicht mehr vom Vec freigegeben werden
        std::mem::forget(vec);

        // Rückgabe einer neuen Matrix mit demselben Speicher
        Matrix {
            data: unsafe { NonNull::new_unchecked(ptr) },
            rows,
            cols,
        }
    }

    /// Konvertiert die Matrix in einen Vec
    pub fn to_vec(&self) -> Vec<f32> {
        let mut vec = Vec::with_capacity(self.flat_len());

        unsafe {
            vec.set_len(self.flat_len());
            self.data.as_ptr().copy_to_nonoverlapping(vec.as_mut_ptr(), self.flat_len());
        }

        vec
    }

    #[inline]
    pub fn into_vec(self) -> Vec<f32> {
        let out = unsafe { Vec::from_raw_parts(self.data.as_ptr(), self.flat_len(), self.flat_len()) };
        forget(self);
        out
    }

    #[inline]
    pub fn zero(&mut self) {
        unsafe { self.data.write_bytes(0, self.flat_len()) };
    }

}


impl Clone for Matrix  {
    fn clone(&self) -> Self {
        let size = self.rows * self.cols;
        let layout = unsafe { Layout::array::<f32>(size).unwrap_unchecked() };

        let data = unsafe {
            let ptr = alloc(layout) as *mut f32;

            //#[cfg(debug_assertions)]
            if ptr.is_null() {
                panic!("Speicherallokation fehlgeschlagen");
            }

            self.data.as_ptr().copy_to_nonoverlapping(ptr, size);
            NonNull::new_unchecked(ptr)
        };
        Self { data, rows: self.rows, cols: self.cols }
    }
}

impl Index<(usize, usize)> for Matrix {
    type Output = f32;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        let (row, col) = index;
        debug_assert!(row < self.rows && col < self.cols, "Index außerhalb der Matrix");

        let idx = row * self.cols + col;
        unsafe { &*self.data.as_ptr().add(idx) }
    }
}

// Implementierung von IndexMut, um das Schreiben über [] zu ermöglichen
impl IndexMut<(usize, usize)> for Matrix {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        let (row, col) = index;
        debug_assert!(row < self.rows && col < self.cols);

        let idx = row * self.cols + col;
        unsafe { &mut *self.data.as_ptr().add(idx) }
    }
}

impl Index<usize> for Matrix {
    type Output = f32;

    fn index(&self, index: usize) -> &Self::Output {
        debug_assert!(self.rows * self.cols > index);

        unsafe { &*self.data.as_ptr().add(index) }
    }
}

// Implementierung von IndexMut, um das Schreiben über [] zu ermöglichen
impl IndexMut<usize> for Matrix {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        debug_assert!(self.rows * self.cols > index);

        unsafe { &mut *self.data.as_ptr().add(index) }
    }
}

impl Drop for Matrix {
    fn drop(&mut self) {
        let size = self.rows * self.cols;
        let layout = unsafe { Layout::array::<f32>(size).unwrap_unchecked() };
        unsafe {
            dealloc(self.data.as_ptr() as _, layout);
        }
    }
}

impl Debug for Matrix {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for i in 0..self.rows {
            for j in 0..self.cols {
                write!(f, "{:.5} ", self[(i, j)])?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl Mul for Matrix {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        debug_assert_eq!(self.cols, rhs.rows, "Matrix-Dimensionen stimmen nicht überein");

        let mut result = Matrix::new(self.rows, rhs.cols);

        for i in 0..self.rows {
            for j in 0..rhs.cols {
                let mut sum = 0.0;
                for k in 0..self.cols {
                    sum += self[(i, k)] * rhs[(k, j)];
                }
                result.set(i, j, sum);
            }
        }

        result
    }
    
}