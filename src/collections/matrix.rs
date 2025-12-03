#![allow(dead_code)]

use std::{
    alloc::{Layout, alloc, dealloc},
    fmt::{Debug, Formatter, Write},
    mem::forget,
    ops::{Index, IndexMut},
    ptr::NonNull,
    slice,
};

use rand;

pub struct Matrix {
    data: NonNull<f32>, // Rohspeicher für die Matrixdaten
    rows: usize,
    cols: usize,
}

impl Matrix {
    #[track_caller]
    pub fn uninit(rows: usize, cols: usize) -> Self {
        let size = rows * cols;
        let layout = unsafe { Layout::array::<f32>(size).unwrap_unchecked() };

        let data = unsafe {
            let ptr = alloc(layout) as *mut f32;

            #[cfg(debug_assertions)]
            if ptr.is_null() {
                panic!("Speicherallokation fehlgeschlagen");
            }
            NonNull::new_unchecked(ptr)
        };

        Self { data, rows, cols }
    }

    #[track_caller]
    pub fn zeros(rows: usize, cols: usize) -> Self {
        let size = rows * cols;
        let layout = unsafe { Layout::array::<f32>(size).unwrap_unchecked() };

        let data = unsafe {
            let ptr = alloc(layout) as *mut f32;

            #[cfg(debug_assertions)]
            if ptr.is_null() {
                panic!("Speicherallokation fehlgeschlagen");
            }

            ptr.write_bytes(0, size);
            NonNull::new_unchecked(ptr)
        };

        Self { data, rows, cols }
    }

    pub fn random(rows: usize, cols: usize, scale: f32) -> Self {
        let mut this = Self::uninit(rows, cols);
        this.as_slice_mut().iter_mut().for_each(|x| *x = rand::random_range(-scale..scale));
        this
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

    #[track_caller]
    pub fn set(&mut self, row: usize, col: usize, value: f32) {
        debug_assert!(row < self.rows && col < self.cols);

        let index = row * self.cols + col;
        unsafe { *self.data.as_ptr().add(index) = value };
    }

    #[track_caller]
    pub fn get(&self, row: usize, col: usize) -> f32 {
        debug_assert!(row < self.rows && col < self.cols);

        let index = row * self.cols + col;
        unsafe { *self.data.as_ptr().add(index) }
    }

    #[track_caller]
    pub fn from_slice(slice: &[f32], rows: usize, cols: usize) -> Self {
        debug_assert_eq!(
            slice.len(),
            rows * cols,
            "Slice-Länge stimmt nicht mit den Matrix-Dimensionen überein"
        );
        let matrix = Self::uninit(rows, cols);

        unsafe {
            slice
                .as_ptr()
                .copy_to_nonoverlapping(matrix.data.as_ptr(), slice.len());
        }

        matrix
    }

    pub fn as_slice(&self) -> &[f32] {
        unsafe { slice::from_raw_parts(self.data.as_ptr(), self.flat_len()) }
    }

    pub fn as_slice_mut(&mut self) -> &mut [f32] {
        unsafe { slice::from_raw_parts_mut(self.data.as_ptr(), self.flat_len()) }
    }

    pub fn copy_from(&mut self, other: &Self) {
        self.as_slice_mut().copy_from_slice(other.as_slice());
    }

    #[track_caller]
    /// Erstellt eine Matrix aus einem Vec
    pub fn from_vec(mut vec: Vec<f32>, rows: usize, cols: usize) -> Self {
        debug_assert_eq!(
            vec.len(),
            rows * cols,
            "Vec-Länge stimmt nicht mit den Matrix-Dimensionen überein"
        );

        // Zeiger aus dem Vec extrahieren
        let ptr = vec.as_mut_ptr();

        // Speicher darf nicht mehr vom Vec freigegeben werden
        std::mem::forget(vec);

        // Rückgabe einer neuen Matrix mit demselben Speicher
        Self {
            data: unsafe { NonNull::new_unchecked(ptr) },
            rows,
            cols,
        }
    }

    #[track_caller]
    /// Erstellt eine Matrix aus einem Box
    pub fn from_box(mut data: Box<[f32]>, rows: usize, cols: usize) -> Self {
        debug_assert_eq!(
            data.len(),
            rows * cols,
            "Vec-Länge stimmt nicht mit den Matrix-Dimensionen überein"
        );

        // Zeiger aus dem Vec extrahieren
        let ptr = data.as_mut_ptr();

        // Speicher darf nicht mehr vom Vec freigegeben werden
        std::mem::forget(data);

        // Rückgabe einer neuen Matrix mit demselben Speicher
        Self {
            data: unsafe { NonNull::new_unchecked(ptr) },
            rows,
            cols,
        }
    }

    #[track_caller]
    /// Konvertiert die Matrix in einen Vec
    pub fn to_vec(&self) -> Vec<f32> {
        #[allow(clippy::uninit_vec)]
        {
            let mut vec = Vec::with_capacity(self.flat_len());

            unsafe {
                vec.set_len(self.flat_len());
                self.data
                    .as_ptr()
                    .copy_to_nonoverlapping(vec.as_mut_ptr(), self.flat_len());
            }
            vec
        }
    }

    #[inline]
    pub fn into_vec(self) -> Vec<f32> {
        let out =
            unsafe { Vec::from_raw_parts(self.data.as_ptr(), self.flat_len(), self.flat_len()) };
        forget(self);
        out
    }

    pub fn split_at_owned(mut self, index: usize) -> (Self, Self) {
        debug_assert!(self.rows > index);
        debug_assert!(index != 0);

        let second = Self {
            data: unsafe { self.data.add(index * self.cols) },
            rows: self.rows - index,
            cols: self.cols,
        };

        self.rows = index;

        (self, second)
    }

    #[track_caller]
    pub fn clear(&mut self) {
        unsafe { self.data.write_bytes(0, self.flat_len()) };
    }

    #[track_caller]
    pub fn scale(&mut self, scale: f32) {
        self.as_slice_mut().iter_mut().for_each(|x| *x *= scale);
    }

    #[track_caller]
    pub fn concat_horizontal(&self, other: &Self) -> Self {
        debug_assert_eq!(self.rows, other.rows);
        let rows = self.rows;
        let cols_left = self.cols;
        let cols_right = other.cols;
        let mut out = Self::uninit(rows, cols_left + cols_right);
        for r in 0..rows {
            out[r][0..cols_left].copy_from_slice(&self[r]);
            out[r][cols_left..cols_left + cols_right].copy_from_slice(&other[r]);
        }
        out
    }

    #[track_caller]
    pub fn hadamard(&self, b: &Self, out: &mut Self) {
        debug_assert_eq!(self.rows, b.rows);
        debug_assert_eq!(self.rows, out.rows);

        debug_assert_eq!(self.cols, b.cols);
        debug_assert_eq!(self.cols, out.cols);

        let a = self.as_slice();
        let b = b.as_slice();
        let out = out.as_slice_mut();

        for i in 0..self.flat_len() {
            out[i] = a[i] * b[i]
        }
    }

    #[track_caller]
    pub fn hadamard_new(&self, b: &Self) -> Self {
        debug_assert_eq!(self.rows, b.rows);
        debug_assert_eq!(self.cols, b.cols);

        let mut output = Self::uninit(self.rows, self.cols);

        let a = self.as_slice();
        let b = b.as_slice();
        let out = output.as_slice_mut();

        for i in 0..self.flat_len() {
            out[i] = a[i] * b[i]
        }
        output
    }

    #[track_caller]
    pub fn mul(&self, other: &Self) -> Self {
        debug_assert_eq!(self.cols, other.rows);

        let mut result = Self::uninit(self.rows, other.cols);

        for i in 0..self.rows {
            for j in 0..other.cols {
                let mut sum = 0.0;
                for k in 0..self.cols {
                    sum += self[(i, k)] * other[(k, j)];
                }
                result.set(i, j, sum);
            }
        }

        result
    }

    #[track_caller]
    pub fn sigmoid(&self) -> Self {
        let mut output = Self::uninit(self.rows(), self.cols());
        let input_data = self.as_slice();
        let output_data = output.as_slice_mut();

        for i in 0..self.flat_len() {
            output_data[i] = 1.0 / (1.0 + (-input_data[i]).exp());
        }
        output
    }

    #[track_caller]
    /// Elementweise tanh-Aktivierung
    pub fn tanh(&self) -> Self {
        let mut output = Self::uninit(self.rows(), self.cols());
        let input_data = self.as_slice();
        let output_data = output.as_slice_mut();

        for i in 0..self.flat_len() {
            output_data[i] = input_data[i].tanh();
        }
        output
    }

    #[track_caller]
    pub fn add_inplace_scaled(&mut self, other: &Self, scale: f32) {
        debug_assert_eq!(self.rows, other.rows);
        debug_assert_eq!(self.cols, other.cols);
        let a = self.as_slice_mut();
        let b = other.as_slice();
        for i in 0..a.len() {
            a[i] += scale * b[i];
        }
    }

    #[track_caller]
    pub fn add_inplace(&mut self, other: &Self) {
        debug_assert_eq!(
            self.rows, other.rows,
            "rows do not match, {} to {}",
            self.rows, other.rows
        );
        debug_assert_eq!(
            self.cols, other.cols,
            "cols do not match, {} to {}",
            self.cols, other.cols
        );

        let a = self.as_slice_mut();
        let b = other.as_slice();

        for i in 0..a.len() {
            a[i] += b[i];
        }
    }

    #[track_caller]
    pub fn sub_inplace(&mut self, other: &Self) {
        debug_assert_eq!(
            self.rows, other.rows,
            "rows do not match, {} to {}",
            self.rows, other.rows
        );
        debug_assert_eq!(
            self.cols, other.cols,
            "cols do not match, {} to {}",
            self.cols, other.cols
        );

        let self_slice = self.as_slice_mut();
        let other_slice = other.as_slice();

        for i in 0..self_slice.len() {
            self_slice[i] -= other_slice[i];
        }
    }

    #[track_caller]
    pub fn add(&self, other: &Self) -> Self {
        debug_assert_eq!(self.rows, other.rows);
        debug_assert_eq!(self.cols, other.cols);

        let this = self.as_slice();
        let other = other.as_slice();
        let out = this.iter().zip(other).map(|(x, y)| x + y).collect();
        Self::from_box(out, self.rows, self.cols)
    }

    #[track_caller]
    pub fn transpose(&self) -> Self {
        let mut out = Self::uninit(self.cols, self.rows);
        for r in 0..self.rows {
            for c in 0..self.cols {
                out[(c, r)] = self[(r, c)];
            }
        }
        out
    }

    #[track_caller]
    pub fn clip(&mut self, min: f32, max: f32) {
        self.as_slice_mut()
            .iter_mut()
            .for_each(|x| *x = x.clamp(min, max))
    }

    #[track_caller]
    pub fn row_mul(&self, row: &[f32], out: &mut [f32]) {
        debug_assert_eq!(self.rows, row.len());
        debug_assert_eq!(self.cols, out.len());

        out.fill(0.0);

        for i in 0..self.rows {
            let weight_row = &self[i];
            let factor = row[i];

            for j in 0..self.cols {
                out[j] += factor * weight_row[j];
            }
        }
    }
}

impl Clone for Matrix {
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
        Self {
            data,
            rows: self.rows,
            cols: self.cols,
        }
    }
}

impl Index<(usize, usize)> for Matrix {
    type Output = f32;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        let (row, col) = index;
        debug_assert!(
            row < self.rows && col < self.cols,
            "Index außerhalb der Matrix"
        );

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
    type Output = [f32];

    fn index(&self, index: usize) -> &Self::Output {
        debug_assert!(
            index < self.rows,
            "Index {index} out of bounds for matrix with {} rows",
            self.rows
        );

        let idx = index * self.cols;
        unsafe {
            let data = self.data.as_ptr().add(idx);
            slice::from_raw_parts(data, self.cols)
        }
    }
}

impl IndexMut<usize> for Matrix {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        debug_assert!(
            index < self.rows,
            "Index {index} out of bounds for matrix with {} rows",
            self.rows
        );

        let idx = index * self.cols;
        unsafe {
            let data = self.data.as_ptr().add(idx);
            slice::from_raw_parts_mut(data, self.cols)
        }
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
        f.write_char('[')?;
        for i in 0..self.rows {
            f.write_char('[')?;
            for j in 0..self.cols {
                write!(f, "{:.5} ", self[(i, j)])?;
            }
            f.write_char(']')?;
            if i != self.rows.wrapping_sub(1) {
                f.write_str(", ")?;
            }
        }
        f.write_char(']')?;
        Ok(())
    }
}
