use core::cell::UnsafeCell;
use core::mem::MaybeUninit;

use crate::event_queue::{MultiProducer, QueueAdapter};

#[cfg(not(test))]
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(test)]
use loom::sync::atomic::{AtomicUsize, Ordering};

struct Slot<T> {
    sequence: AtomicUsize,
    data: UnsafeCell<MaybeUninit<T>>,
}

pub struct MpmcBoundedQueue<T, const SIZE: usize> {
    buffer: [Slot<T>; SIZE],
    enqueue_pos: AtomicUsize,
    dequeue_pos: AtomicUsize,
}

impl<T, const SIZE: usize> Default for MpmcBoundedQueue<T, SIZE> {
    fn default() -> Self {
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
}

impl<T, const SIZE: usize> MpmcBoundedQueue<T, SIZE> {
    const BUFFER_MASK: usize = SIZE - 1;
}

impl<T, const SIZE: usize> QueueAdapter<T> for MpmcBoundedQueue<T, SIZE> {
    fn enqueue(&self, data: T) -> Result<(), T> {
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
                return Err(data);
            } else {
                pos = self.enqueue_pos.load(Ordering::Relaxed);
            }
        }
    }

    fn dequeue(&self) -> Option<T> {
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

impl<T, const SIZE: usize> Drop for MpmcBoundedQueue<T, SIZE> {
    fn drop(&mut self) {
        while self.dequeue().is_some() {}
    }
}

impl<T, const N: usize> MultiProducer for MpmcBoundedQueue<T, N> {}

unsafe impl<T: Send, const N: usize> Send for MpmcBoundedQueue<T, N> {}
unsafe impl<T: Send, const N: usize> Sync for MpmcBoundedQueue<T, N> {}

#[cfg(test)]
mod tests {
    use super::MpmcBoundedQueue;
    use crate::event_queue::QueueAdapter;
    use loom::sync::Arc;
    use loom::sync::atomic::{AtomicUsize, Ordering};
    use loom::thread;

    #[test]
    fn enqueue_dequeue_roundtrip() {
        loom::model(|| {
            let queue = MpmcBoundedQueue::<u32, 4>::default();

            assert_eq!(queue.dequeue(), None);
            assert_eq!(queue.enqueue(7), Ok(()));
            assert_eq!(queue.dequeue(), Some(7));
            assert_eq!(queue.dequeue(), None);
        });
    }

    #[test]
    fn queue_full_and_empty_conditions() {
        loom::model(|| {
            let queue = MpmcBoundedQueue::<u32, 2>::default();

            assert_eq!(queue.enqueue(1), Ok(()));
            assert_eq!(queue.enqueue(2), Ok(()));
            assert_eq!(queue.enqueue(3), Err(3));

            assert_eq!(queue.dequeue(), Some(1));
            assert_eq!(queue.dequeue(), Some(2));
            assert_eq!(queue.dequeue(), None);
        });
    }

    #[test]
    fn wraparound_behavior() {
        loom::model(|| {
            let queue = MpmcBoundedQueue::<u32, 4>::default();

            for i in 0..4 {
                assert_eq!(queue.enqueue(i), Ok(()));
            }

            assert_eq!(queue.dequeue(), Some(0));
            assert_eq!(queue.dequeue(), Some(1));

            assert_eq!(queue.enqueue(4), Ok(()));
            assert_eq!(queue.enqueue(5), Ok(()));
            assert_eq!(queue.enqueue(6), Err(6));

            assert_eq!(queue.dequeue(), Some(2));
            assert_eq!(queue.dequeue(), Some(3));
            assert_eq!(queue.dequeue(), Some(4));
            assert_eq!(queue.dequeue(), Some(5));
            assert_eq!(queue.dequeue(), None);
        });
    }

    #[test]
    fn loom_spsc_single_item() {
        loom::model(|| {
            let queue = Arc::new(MpmcBoundedQueue::<usize, 2>::default());

            let producer_queue = Arc::clone(&queue);
            let producer = thread::spawn(move || {
                while producer_queue.enqueue(42).is_err() {
                    thread::yield_now();
                }
            });

            let consumer = thread::spawn(move || {
                loop {
                    if let Some(value) = queue.dequeue() {
                        assert_eq!(value, 42);
                        break;
                    }
                    thread::yield_now();
                }
            });

            producer.join().expect("producer thread panicked");
            consumer.join().expect("consumer thread panicked");
        });
    }

    #[test]
    fn loom_mpsc_all_items_consumed_once() {
        loom::model(|| {
            let queue = Arc::new(MpmcBoundedQueue::<usize, 2>::default());
            let consumed_count = Arc::new(AtomicUsize::new(0));
            let seen_mask = Arc::new(AtomicUsize::new(0));

            let producer1_queue = Arc::clone(&queue);
            let producer1 = thread::spawn(move || {
                while producer1_queue.enqueue(0).is_err() {
                    thread::yield_now();
                }
            });

            let producer2_queue = Arc::clone(&queue);
            let producer2 = thread::spawn(move || {
                while producer2_queue.enqueue(1).is_err() {
                    thread::yield_now();
                }
            });

            let consumer_queue = Arc::clone(&queue);
            let consumer_count = Arc::clone(&consumed_count);
            let consumer_mask = Arc::clone(&seen_mask);
            let consumer = thread::spawn(move || {
                while consumer_count.load(Ordering::Acquire) < 2 {
                    if let Some(value) = consumer_queue.dequeue() {
                        let bit = 1 << value;
                        let previous = consumer_mask.fetch_or(bit, Ordering::AcqRel);
                        assert_eq!(previous & bit, 0, "value dequeued more than once");
                        consumer_count.fetch_add(1, Ordering::AcqRel);
                    } else {
                        thread::yield_now();
                    }
                }
            });

            producer1.join().expect("producer1 thread panicked");
            producer2.join().expect("producer2 thread panicked");
            consumer.join().expect("consumer thread panicked");

            assert_eq!(consumed_count.load(Ordering::Acquire), 2);
            assert_eq!(seen_mask.load(Ordering::Acquire), 0b11);
            assert_eq!(queue.dequeue(), None);
        });
    }

    use core::cell::Cell;

    #[derive(Debug)]
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
        loom::model(|| {
            let counter = Cell::new(0);

            {
                let queue = MpmcBoundedQueue::<DropCounter, 4>::default();

                queue
                    .enqueue(DropCounter { counter: &counter })
                    .expect("Queue unexpectedly full");
                queue
                    .enqueue(DropCounter { counter: &counter })
                    .expect("Queue unexpectedly full");

                assert_eq!(counter.get(), 0);
            }

            // Nothing extra dropped
            assert_eq!(counter.get(), 2);
        });
    }

    #[test]
    fn drop_consumed() {
        loom::model(|| {
            let counter = Cell::new(0);

            {
                let queue = MpmcBoundedQueue::<DropCounter, 4>::default();

                queue
                    .enqueue(DropCounter { counter: &counter })
                    .expect("Queue unexpectedly full");
                queue
                    .enqueue(DropCounter { counter: &counter })
                    .expect("Queue unexpectedly full");

                let _a = queue.dequeue().unwrap();
                let _b = queue.dequeue().unwrap();

                assert_eq!(counter.get(), 0);
            }

            // Nothing extra dropped
            assert_eq!(counter.get(), 2);
        });
    }
}
