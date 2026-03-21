use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

pub trait QueueAdapter<T> {
    fn enqueue(&self, data: T) -> Result<(), T>;
    fn dequeue(&self) -> Option<T>;
}

pub trait MultiProducer {}

pub struct EventProducer<'a, E, Q: QueueAdapter<E>> {
    inner: &'a Q,
    _pd: PhantomData<E>,
}

impl<'a, E, Q: QueueAdapter<E>> EventProducer<'a, E, Q> {
    pub fn enqueue(&self, event: E) -> Result<(), E> {
        self.inner.enqueue(event)
    }
}

impl<'a, E, Q: QueueAdapter<E> + MultiProducer> Clone for EventProducer<'a, E, Q> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            _pd: PhantomData::<E>,
        }
    }
}

pub struct EventConsumer<'a, E, Q: QueueAdapter<E>> {
    inner: &'a Q,
    _pd: PhantomData<E>,
}

impl<'a, E, Q: QueueAdapter<E>> EventConsumer<'a, E, Q> {
    pub(crate) fn dequeue(&self) -> Option<E> {
        self.inner.dequeue()
    }
}

pub struct Mailbox<E, Q: QueueAdapter<E>> {
    inner: Q,
    split: AtomicBool,
    _pd: PhantomData<E>,
}

impl<E, Q: QueueAdapter<E>> Mailbox<E, Q> {
    pub const fn new(queue: Q) -> Self {
        Self {
            inner: queue,
            split: AtomicBool::new(false),
            _pd: PhantomData::<E>,
        }
    }

    pub fn split(&self) -> Option<(EventProducer<'_, E, Q>, EventConsumer<'_, E, Q>)> {
        self.split
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()?;

        Some((
            EventProducer::<E, Q> {
                inner: &self.inner,
                _pd: PhantomData::<E>,
            },
            EventConsumer::<E, Q> {
                inner: &self.inner,
                _pd: PhantomData::<E>,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{Mailbox, MultiProducer, QueueAdapter};
    use core::cell::Cell;

    struct TestQueue {
        value: Cell<Option<u32>>,
    }

    impl TestQueue {
        fn new() -> Self {
            Self {
                value: Cell::new(None),
            }
        }
    }

    impl QueueAdapter<u32> for TestQueue {
        fn enqueue(&self, data: u32) -> Result<(), u32> {
            if self.value.get().is_some() {
                return Err(data);
            }

            self.value.set(Some(data));
            Ok(())
        }

        fn dequeue(&self) -> Option<u32> {
            let value = self.value.get();
            self.value.set(None);
            value
        }
    }

    impl MultiProducer for TestQueue {}

    #[test]
    fn mailbox_split_only_succeeds_once() {
        let mailbox = Mailbox::<u32, _>::new(TestQueue::new());

        assert!(mailbox.split().is_some());
        assert!(mailbox.split().is_none());
    }

    #[test]
    fn cloned_producers_can_enqueue() {
        let mailbox = Mailbox::<u32, _>::new(TestQueue::new());
        let (producer, consumer) = mailbox.split().expect("first split should succeed");
        let producer_clone = producer.clone();

        assert_eq!(producer.enqueue(7), Ok(()));
        assert_eq!(producer_clone.enqueue(8), Err(8));
        assert_eq!(consumer.dequeue(), Some(7));
    }
}
