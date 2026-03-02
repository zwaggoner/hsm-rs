use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};

struct Slot<T> {
    sequence: AtomicUsize,
    data: UnsafeCell<MaybeUninit<T>>,
}

pub(crate) struct MpmcBoundedQueue<T, const SIZE: usize> {
    buffer: [Slot<T>; SIZE],
    enqueue_pos: AtomicUsize,
    dequeue_pos: AtomicUsize,
}

impl<T, const SIZE: usize> MpmcBoundedQueue<T, SIZE> {
    const BUFFER_MASK: usize = SIZE - 1;

    pub(crate) fn new() -> Self {
        assert!(SIZE >= 2, "Queue size must be at least two elements");
        assert!(SIZE.is_power_of_two(), "Queue size must be a power of two");
        MpmcBoundedQueue::<T, SIZE> {
            buffer: core::array::from_fn(|i| Slot::<T> {
                sequence: AtomicUsize::new(i),
                data: UnsafeCell::new(MaybeUninit::uninit()),
            }),
            enqueue_pos: AtomicUsize::new(0),
            dequeue_pos: AtomicUsize::new(0),
        }
    }

    pub(crate) fn enqueue(&self, data: T) -> Result<(), ()> {
        let mut pos = self.enqueue_pos.load(Ordering::Relaxed);

        loop {
            let slot = &self.buffer[pos & Self::BUFFER_MASK];
            let seq = slot.sequence.load(Ordering::Acquire);
            let dif: isize = seq as isize - pos as isize;

            if dif == 0 {
                match self.enqueue_pos.compare_exchange_weak(
                    pos,
                    pos + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        unsafe {
                            (*slot.data.get()).write(data);
                        }

                        slot.sequence.store(pos + 1, Ordering::Release);
                        return Ok(());
                    }
                    Err(new_pos) => pos = new_pos,
                }
            } else if dif < 0 {
                return Err(());
            } else {
                pos = self.enqueue_pos.load(Ordering::Relaxed);
            }
        }
    }

    pub(crate) fn dequeue(&self) -> Option<T> {
        let mut pos = self.dequeue_pos.load(Ordering::Relaxed);

        loop {
            let slot = &self.buffer[pos & Self::BUFFER_MASK];
            let seq = slot.sequence.load(Ordering::Acquire);

            let dif: isize = seq as isize - (pos + 1) as isize;

            if dif == 0 {
                match self.dequeue_pos.compare_exchange_weak(
                    pos,
                    pos + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        let data = unsafe { (*slot.data.get()).assume_init_read() };

                        slot.sequence.store(pos + SIZE, Ordering::Release);
                        return Some(data);
                    }
                    Err(new_pos) => pos = new_pos,
                }
            } else if dif < 0 {
                return None;
            } else {
                pos = self.dequeue_pos.load(Ordering::Relaxed);
            }
        }
    }
}

// Ensure the queue can be safely shared across thread boundaries
unsafe impl<T: Copy + Send, const N: usize> Send for MpmcBoundedQueue<T, N> {}

// The queue itself can be shared immutably via atomic operations for pushing
// Only the consumer needs exclusive access for popping
unsafe impl<T: Copy + Sync, const N: usize> Sync for MpmcBoundedQueue<T, N> {}
