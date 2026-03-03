use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};

pub(crate) struct FixedVec<T: Copy, const MAX_DEPTH: usize = 32> {
    arr: [MaybeUninit<T>; MAX_DEPTH],
    len: usize,
}

impl<T: Copy, const MAX_DEPTH: usize> FixedVec<T, MAX_DEPTH> {
    pub(crate) fn new() -> Self {
        FixedVec {
            arr: [const { MaybeUninit::uninit() }; MAX_DEPTH],
            len: 0,
        }
    }

    pub(crate) fn push(&mut self, elem: T) -> Result<(), ()> {
        if self.len < MAX_DEPTH {
            self.arr[self.len].write(elem);
            self.len += 1;
            return Ok(());
        }

        return Err(());
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;
        Some(unsafe { self.arr[self.len].assume_init_read() })
    }
}

impl<T: Copy, const MAX_DEPTH: usize> Deref for FixedVec<T, MAX_DEPTH> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.arr.as_ptr() as *const T, self.len) }
    }
}

impl<T: Copy, const MAX_DEPTH: usize> DerefMut for FixedVec<T, MAX_DEPTH> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.arr.as_mut_ptr() as *mut T, self.len) }
    }
}
