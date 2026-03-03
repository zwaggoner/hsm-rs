use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::ptr;
use core::slice;

pub(crate) struct FixedVec<T, const MAX_DEPTH: usize = 32> {
    arr: [MaybeUninit<T>; MAX_DEPTH],
    len: usize,
}

impl<T, const MAX_DEPTH: usize> FixedVec<T, MAX_DEPTH> {
    pub(crate) const fn new() -> Self {
        Self {
            arr: [const { MaybeUninit::uninit() }; MAX_DEPTH],
            len: 0,
        }
    }

    pub(crate) fn push(&mut self, elem: T) -> Result<(), ()> {
        if self.len == MAX_DEPTH {
            return Err(());
        }

        self.arr[self.len].write(elem);
        self.len += 1;
        Ok(())
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;

        Some(unsafe { self.arr[self.len].assume_init_read() })
    }

    pub(crate) fn clear(&mut self) {
        unsafe {
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(
                self.arr.as_mut_ptr() as *mut T,
                self.len,
            ));
        }

        self.len = 0;
    }
}

impl<T, const MAX_DEPTH: usize> Drop for FixedVec<T, MAX_DEPTH> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T, const MAX_DEPTH: usize> Deref for FixedVec<T, MAX_DEPTH> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.arr.as_ptr() as *const T, self.len) }
    }
}

impl<T, const MAX_DEPTH: usize> DerefMut for FixedVec<T, MAX_DEPTH> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.arr.as_mut_ptr() as *mut T, self.len) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pop() {
        let mut v: FixedVec<i32, 3> = FixedVec::new();

        assert_eq!(v.len(), 0);
        assert_eq!(v.pop(), None);

        const TEST_LEN : usize = 3;
        let test : [i32; TEST_LEN] = [1, 2, 3];

        for val in test {
            v.push(val).unwrap();
        }

        let mut expected_size = TEST_LEN;

        assert_eq!(v.len(), expected_size);
        assert_eq!(&*v, &test);

        for i in 0..TEST_LEN {
            assert_eq!(v.pop(), Some(test[TEST_LEN - i - 1]));

            expected_size -= 1;
            assert_eq!(v.len(), expected_size);
        }

        assert_eq!(v.pop(), None);
    }
}
