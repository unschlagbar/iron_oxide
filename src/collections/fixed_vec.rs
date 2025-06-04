use std::alloc::{alloc, Layout};
use std::fmt::{self};
use std::marker::PhantomData;
use std::mem::{forget, ManuallyDrop};
use std::ptr::{self, NonNull};
use std::ops::{Add, AddAssign, Index, IndexMut};

pub struct FixedVec<T> {
    data: NonNull<T>, // Rohspeicher für die Daten
    len: usize,    // Feste Länge des Vektors
}

impl<T> FixedVec<T> {

    #[inline]
    pub fn new(len: usize) -> Self {
        debug_assert!(len > 0, "Len must not be 0");
        
        // Allokation des Speichers
        let layout = Layout::array::<T>(len).expect("Ungültiges Layout");
        let data = unsafe {
            let ptr = alloc(layout) as *mut T;
            if ptr.is_null() {
                panic!("Speicherallokation fehlgeschlagen");
            }

            ptr::write_bytes(ptr, 0, len);

            NonNull::new_unchecked(ptr)
        };

        FixedVec { data, len }
    }

    #[inline]
    #[track_caller]
    pub fn uninit(len: usize) -> Self {
        debug_assert!(len > 0, "Len must not be 0");
        
        // Allokation des Speicher
        let layout = Layout::array::<T>(len).expect("Ungültiges Layout");
        let data = unsafe {
            let ptr = alloc(layout) as *mut T;
            if ptr.is_null() {
                panic!("Speicherallokation fehlgeschlagen");
            }
            NonNull::new_unchecked(ptr)
        };

        FixedVec { data, len }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[inline]
    #[track_caller]
    pub fn set(&mut self, index: usize, value: T) {
        debug_assert!(index < self.len, "Index out of bounds: the len is {} but the index is {}", self.len, index);
        unsafe {
            self.data.as_ptr().add(index).write(value);
        }
    }

    #[inline]
    #[track_caller]
    pub fn get(&self, index: usize) -> &T {
        debug_assert!(index < self.len, "Index out of bounds: the len is {} but the index is {}", self.len, index);
        unsafe {
            &*self.data.as_ptr().add(index)
        }
    }

    #[inline]
    #[track_caller]
    pub fn get_mut(&mut self, index: usize) -> &mut T {
        debug_assert!(index < self.len, "Index out of bounds: the len is {} but the index is {}", self.len, index);
        unsafe {
            &mut *self.data.as_ptr().add(index)
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(self.data.as_ptr(), self.len)
        }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe {
            std::slice::from_raw_parts_mut(self.data.as_ptr(), self.len)
        }
    }

    #[inline]
    ///slice should be no longer used
    pub unsafe fn from_slice(slice: &[T]) -> Self {
        let len = slice.len();
        let layout = Layout::array::<T>(len).expect("Ungültiges Layout");
        let data = unsafe {
            let ptr = alloc(layout) as *mut T;
            if ptr.is_null() {
                panic!("Speicherallokation fehlgeschlagen");
            }

            ptr.copy_from_nonoverlapping(slice.as_ptr(), len);

            NonNull::new_unchecked(ptr)
        };

        FixedVec { data, len }
    }

    #[inline]
    pub const fn as_ptr(&self) -> *const T {
        self.data.as_ptr()
    }

    #[inline]
    pub unsafe fn from_raw_parts(ptr: *mut T, len: usize) -> Self {
        debug_assert!(len > 0, "Len must not be 0");

        let data = NonNull::new(ptr).expect("Pointer must not be null");

        FixedVec { data, len }
    }

    #[inline]
    pub fn iter<'a>(&self) -> IntoIter<'a, T> {
        unsafe {
            let begin = self.data.as_ptr();
            let end = begin.add(self.len) as *const T;
            IntoIter { ptr: begin, end, _marker: PhantomData }
        }
    }

    #[inline]
    pub fn iter_mut<'a>(&self) -> IterMut<'a, T> {
        unsafe {
            let begin = self.data.as_ptr();
            let end = begin.add(self.len) as *mut T;
            IterMut { ptr: begin, end, _marker: PhantomData }
        }
    }

    #[inline]
    pub fn last(&self) -> Option<&T> {
        if self.len == 0 {
            None
        } else {
            Some(unsafe { &*self.data.as_ptr().add(self.len - 1) })
        }
    }

    #[inline]
    pub fn last_mut(&self) -> Option<&mut T> {
        if self.len == 0 {
            None
        } else {
            Some(unsafe { &mut *self.data.as_ptr().add(self.len - 1) })
        }
    }
}

impl<T: Default> FixedVec<T> {
    #[inline]
    pub fn zero(&mut self) {
        unsafe { 
            for i in 0..self.len {
                self.data.as_ptr().add(i).write(T::default());
            }
        };
    }

    #[inline]
    pub fn default(len: usize) -> Self {
        debug_assert!(len > 0, "Len must not be 0");
        
        // Allokation des Speichers
        let layout = Layout::array::<T>(len).expect("Ungültiges Layout");
        let data = unsafe {
            let ptr = alloc(layout) as *mut T;
            if ptr.is_null() {
                panic!("Speicherallokation fehlgeschlagen");
            }

            for i in 0..len {
                ptr.add(i).write(T::default());
            }

            NonNull::new_unchecked(ptr)
        };


        FixedVec { data, len }
    }
}

impl<T> IntoIterator for FixedVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        unsafe {
            let vec = Vec::from_raw_parts(self.data.as_ptr(), self.len, self.len);
            forget(self);
            vec.into_iter()
        }
    }
}

impl<'a, T> IntoIterator for &'a FixedVec<T> {
    type Item = &'a T;
    type IntoIter = IntoIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut FixedVec<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}


impl<T: Copy> FixedVec<T> {
    // Allokiert den Speicher und initialisiert den Vektor mit einem Wert
    pub fn with_value(len: usize, value: T) -> Self {
        debug_assert!(len > 0, "FixedVec muss eine nicht-leere Länge haben");

        let layout = Layout::array::<T>(len).expect("Ungültiges Layout");
        let data = unsafe {
            let ptr = alloc(layout) as *mut T;
            if ptr.is_null() {
                panic!("Speicherallokation fehlgeschlagen");
            }

            // Initialisieren der Werte mit dem angegebenen Standardwert
            for i in 0..len {
                ptr.add(i).write(value); // Schreibe 'value' an alle Positionen
            }

            NonNull::new_unchecked(ptr)
        };

        FixedVec { data, len }
    }

}

impl<T: Copy> From<&[T]> for FixedVec<T> {
    fn from(slice: &[T]) -> Self {
        unsafe { FixedVec::from_slice(slice) }
    }
}

impl<T: Copy> From<&Vec<T>> for FixedVec<T> {
    fn from(vec: &Vec<T>) -> Self {
        let len = vec.len();
        let layout = Layout::array::<T>(len).expect("Ungültiges Layout");
        
        let data = unsafe {
            let ptr = alloc(layout) as *mut T;
            if ptr.is_null() {
                panic!("Speicherallokation fehlgeschlagen");
            }

            vec.as_ptr().copy_to_nonoverlapping(ptr, len);

            NonNull::new_unchecked(ptr)
        };

        FixedVec { data, len }
    }
}

impl<T: Copy> Into<Vec<T>> for FixedVec<T> {
    fn into(self) -> Vec<T> {
        let me = ManuallyDrop::new(self);
        let ptr = me.data.as_ptr();
        let length = me.len;
        let capacity = length;
        
        unsafe { Vec::from_raw_parts(ptr, length, capacity) }
    }
}

impl<T: Copy> Into<Vec<T>> for &FixedVec<T> {
    fn into(self) -> Vec<T> {
        let len = self.len();
        let layout = Layout::array::<T>(len).expect("Ungültiges Layout");
        
        let ptr = unsafe {
            let ptr = alloc(layout) as *mut T;
            if ptr.is_null() {
                panic!("Speicherallokation fehlgeschlagen");
            }

            self.as_ptr().copy_to_nonoverlapping(ptr, len);
            ptr
        };

        unsafe { Vec::from_raw_parts(ptr, len, len) }
    }
}

// Implementierung für `From<&[T; N]>` für statische Arrays
impl<T: Copy, const N: usize> From<&[T; N]> for FixedVec<T> {
    fn from(array: &[T; N]) -> Self {
        unsafe { FixedVec::from_slice(array) }
    }
}

impl<T> AsRef<[T]> for FixedVec<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice() // Nutzt die bereits implementierte Methode
    }
}

impl<T> AsMut<[T]> for FixedVec<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice() // Nutzt die bereits implementierte Methode
    }
}


// Ermöglicht den Zugriff über Index-Operatoren
impl<T> Index<usize> for FixedVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
    }
}

impl<T> IndexMut<usize> for FixedVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index)
    }
}

impl<T: Clone> Clone for FixedVec<T> {
    fn clone(&self) -> Self {
        let len = self.len();
        let layout = Layout::array::<T>(len).expect("Ungültiges Layout");
        
        let data = unsafe {
            let ptr = alloc(layout) as *mut T;
            if ptr.is_null() {
                panic!("Speicherallokation fehlgeschlagen");
            }

            self.as_ptr().copy_to_nonoverlapping(ptr, len);
            NonNull::new_unchecked(ptr)
        };

        FixedVec { data, len }
    }
}

impl<T> Drop for FixedVec<T> {
    fn drop(&mut self) {
        if self.len > 0 {
            unsafe {
                std::ptr::drop_in_place(ptr::slice_from_raw_parts_mut(self.data.as_ptr(), self.len));
                std::alloc::dealloc(
                    self.data.as_ptr() as *mut u8,
                    std::alloc::Layout::array::<T>(self.len).expect("Invalid layout"),
                );
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for FixedVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&mut self.iter()).finish()
    }
}

impl<T: Add<Output = T> + Copy> Add for FixedVec<T> {

    type Output = FixedVec<T>;

    #[inline]
    fn add(self, rsh: Self) -> Self::Output {
        debug_assert!(self.len() == rsh.len(), "FixedVecs must have the same length");

        let mut sum = FixedVec::uninit(self.len());
        for i in 0..self.len() {
            sum.set(i, *self.get(i) + *rsh.get(i));
        }
        sum
    }
    
}

impl<T: AddAssign + Copy> AddAssign for FixedVec<T>  {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        debug_assert!(self.len() == rhs.len(), "Length mismatch");

        for i in 0..self.len() {
            self[i] += rhs[i];
        }
    }
}


pub struct IntoIter<'a, T> {
    ptr: *const T,
    end: *const T,
    _marker: std::marker::PhantomData<&'a T>,
}

impl<'a, T> Iterator for IntoIter<'a, T> {
    type Item = &'a T;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr == self.end {

            None
        } else {
            let item = unsafe { &*self.ptr };
            self.ptr = unsafe { self.ptr.add(1) };
            Some(item)
        }
    }
}

pub struct IterMut<'a, T> {
    ptr: *mut T,
    end: *mut T,
    _marker: std::marker::PhantomData<&'a T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr == self.end {

            None
        } else {
            let item = unsafe { &mut *self.ptr };
            self.ptr = unsafe { self.ptr.add(1) };
            Some(item)
        }
    }
}



#[macro_export]
macro_rules! fixed_vec {
    // Regel für die Notation `fixed_vec![value; len]`
    ($x:expr; $len:expr) => {{
        let len = $len;
        let value = $x;
        FixedVec::with_value(len, value)
    }};

    // Regel für die Notation `fixed_vec![v1, v2, v3, ...]`
    ($($x:expr),+ $(,)?) => {{
        let slice = &[$($x),+];
        FixedVec::from(slice)
    }};
}