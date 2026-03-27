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
                self.arr.as_mut_ptr().cast::<T>(),
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
        unsafe { slice::from_raw_parts(self.arr.as_ptr().cast::<T>(), self.len) }
    }
}

impl<T, const MAX_DEPTH: usize> DerefMut for FixedVec<T, MAX_DEPTH> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.arr.as_mut_ptr().cast::<T>(), self.len) }
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

        const TEST_LEN: usize = 3;
        let test: [i32; TEST_LEN] = [1, 2, 3];

        for val in test {
            v.push(val).unwrap();
        }

        let mut expected_size = TEST_LEN;

        assert_eq!(v.len(), expected_size);
        assert_eq!(&*v, &test);

        for val in test.iter().rev() {
            assert_eq!(v.pop(), Some(*val));

            expected_size -= 1;
            assert_eq!(v.len(), expected_size);
        }

        assert_eq!(v.pop(), None);
    }

    #[test]
    fn overflow() {
        let mut v: FixedVec<i32, 1> = FixedVec::new();

        assert!(v.push(1).is_ok());
        assert!(v.push(2).is_err());

        assert_eq!(v.len(), 1);
    }

    #[test]
    fn clear() {
        let mut v: FixedVec<i32, 4> = FixedVec::new();

        v.push(1).unwrap();
        v.push(2).unwrap();

        v.clear();

        assert_eq!(v.len(), 0);
        assert_eq!(v.pop(), None);
    }

    use core::cell::Cell;

    struct DropCounter<'a> {
        counter: &'a Cell<usize>,
    }

    impl<'a> Drop for DropCounter<'a> {
        fn drop(&mut self) {
            let v = self.counter.get();
            self.counter.set(v + 1);
        }
    }

    #[test]
    fn drop() {
        let counter = Cell::new(0);

        {
            let mut v: FixedVec<DropCounter, 4> = FixedVec::new();

            v.push(DropCounter { counter: &counter }).unwrap();
            v.push(DropCounter { counter: &counter }).unwrap();

            assert_eq!(counter.get(), 0);
        }

        // Nothing extra dropped
        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn drop_on_clear() {
        let counter = Cell::new(0);

        {
            let mut v: FixedVec<DropCounter, 4> = FixedVec::new();

            v.push(DropCounter { counter: &counter }).unwrap();
            v.push(DropCounter { counter: &counter }).unwrap();

            v.clear();

            assert_eq!(counter.get(), 2);
        }

        // Nothing extra dropped
        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn deref_mut() {
        let mut v: FixedVec<i32, 4> = FixedVec::new();

        v.push(1).unwrap();
        v.push(2).unwrap();

        v[0] = 10;

        assert_eq!(&*v, &[10, 2]);
    }

    #[test]
    fn reverse() {
        let mut v: FixedVec<i32, 3> = FixedVec::new();

        const TEST_LEN: usize = 3;
        let test: [i32; TEST_LEN] = [1, 2, 3];

        for val in test {
            v.push(val).unwrap();
        }

        v.reverse();

        for (val, val_rev) in core::iter::zip(v.iter(), test.iter().rev()) {
            assert_eq!(*val, *val_rev);
        }
    }
}
