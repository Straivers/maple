use std::mem::MaybeUninit;

/// A fixed-capacity vector of T. Attempting to add elements beyond its
/// capacity will cause a panic.
pub struct ArrayVec<T, const N: usize> {
    // We just need the allocated space, don't really care about what's in it.
    array: MaybeUninit<[T; N]>,
    length: u32,
}

impl<T, const N: usize> ArrayVec<T, N> {
    /// Create a new fized-capacity vector on the stack.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of elements in the vector.
    pub fn len(&self) -> usize {
        self.length as usize
    }

    /// Shorthand for `len() == 0`
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// The statically determined capacity for the vector.
    pub fn capacity(&self) -> usize {
        N
    }

    /// Retrieves a pointer to the first element of the vector. The returned
    /// pointer will be null if `is_empty()`.
    pub fn as_ptr(&self) -> *const T {
        if self.is_empty() {
            std::ptr::null()
        } else {
            unsafe { self.as_ptr_unchecked() }
        }
    }

    /// Retrieves a pointer to the first element of the vector without checking
    /// for the vector's length.
    unsafe fn as_ptr_unchecked(&self) -> *const T {
        (*self.array.as_ptr()).as_ptr()
    }

    /// Retrieves a mutable pointer to the first element of the vector. The
    /// returned pointer will be null if `is_empty()`.
    pub fn as_mut_ptr(&mut self) -> *mut T {
        if self.is_empty() {
            std::ptr::null_mut()
        } else {
            unsafe { self.as_mut_ptr_unchecked() }
        }
    }

    /// Retrieves a mutable pionter to the first element of the vector without
    /// checking the vector's length.
    unsafe fn as_mut_ptr_unchecked(&mut self) -> *mut T {
        (*self.array.as_mut_ptr()).as_mut_ptr()
    }

    /// Produces a slice spanning the entire vector.
    pub fn as_slice(&self) -> &[T] {
        if self.is_empty() {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.as_ptr_unchecked(), self.length as usize) }
        }
    }

    /// Produces a mutable slice spanning the entire vector.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        if self.is_empty() {
            &mut []
        } else {
            unsafe {
                std::slice::from_raw_parts_mut(self.as_mut_ptr_unchecked(), self.length as usize)
            }
        }
    }

    /// Pushes a new element to the back of the vector.
    ///
    /// # Panics
    /// This function will panic if the vector is at capacity.
    pub fn push(&mut self, value: T) {
        if (self.length as usize) < N {
            unsafe {
                self.as_mut_ptr_unchecked()
                    .add(self.length as usize)
                    .write(value)
            };

            self.length += 1;
        } else {
            panic!("ArrayVec out of capacity");
        }
    }
}

impl<T, const N: usize> Default for ArrayVec<T, N> {
    fn default() -> Self {
        Self {
            array: MaybeUninit::uninit(),
            length: 0,
        }
    }
}

impl<T, const N: usize> Drop for ArrayVec<T, N> {
    fn drop(&mut self) {
        for element in self.as_mut_slice() {
            unsafe { std::ptr::drop_in_place(element) };
        }
    }
}

impl<T: std::fmt::Debug, const N: usize> std::fmt::Debug for ArrayVec<T, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.debug_list().entries(self.as_slice()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_vec_init() {
        let mut vec = ArrayVec::<u32, 3>::new();

        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 3);

        assert_eq!(vec.as_slice().len(), 0);
        assert_eq!(vec.as_mut_slice().len(), 0);
    }

    #[test]
    fn array_vec_push_drop() {
        static mut K: u32 = 0;

        #[derive(Debug)]
        struct T(u32);

        impl Drop for T {
            fn drop(&mut self) {
                unsafe { K += 1 };
                println!("drop");
            }
        }

        impl PartialEq<u32> for T {
            fn eq(&self, other: &u32) -> bool {
                self.0 == *other
            }
        }

        let mut vec = ArrayVec::<T, 4>::new();

        vec.push(T(0));
        assert_eq!(vec.len(), 1);
        assert_eq!(vec.capacity(), 4);
        assert_eq!(vec.as_slice(), [0]);
        assert_eq!(vec.as_mut_slice(), [0]);

        vec.as_mut_slice()[0] = T(1);
        assert_eq!(vec.len(), 1);
        assert_eq!(vec.capacity(), 4);
        assert_eq!(vec.as_slice(), [1]);
        assert_eq!(vec.as_mut_slice(), [1]);

        vec.push(T(2));
        assert_eq!(vec.len(), 2);
        assert_eq!(vec.capacity(), 4);
        assert_eq!(vec.as_slice(), [1, 2]);
        assert_eq!(vec.as_mut_slice(), [1, 2]);

        std::mem::drop(vec);

        unsafe { assert_eq!(K, 3) };
    }
}
