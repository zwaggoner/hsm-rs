use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

/// `QueueAdapter` trait is made available such that any underlying queue that implements it can be
/// used as an event queue in the rsm framework.
/// `QueueAdapter` queues are required to implement interior mutability such that the
/// consumer/producer split can be managed cleanly by the framework no matter the underlying queue
pub trait QueueAdapter<T> {
    /// Enqueues an item to the underlying queue
    /// # Errors
    /// If the queue is unable to enqueue the data, it will return an error.
    fn enqueue(&self, data: T) -> Result<(), T>;
    fn dequeue(&self) -> Option<T>;
}

pub trait MultiProducer {}

pub struct EventQueue<E, Q: QueueAdapter<E>> {
    inner: Q,
    producer_split: AtomicBool,
    consumer_split: AtomicBool,
    _pd: PhantomData<E>,
}

pub struct EventProducer<'a, E, Q: QueueAdapter<E>> {
    inner: &'a Q,
    _pd: PhantomData<E>
}

impl<E, Q: QueueAdapter<E>> EventQueue<E, Q> {
    pub const fn new(queue: Q) -> Self {
        Self {
            inner: queue,
            producer_split: AtomicBool::new(false),
            consumer_split: AtomicBool::new(false),
            _pd: PhantomData,
        }
    }

    pub fn producer(&self) -> Option<EventProducer<'_, E, Q>> {
        self.producer_split
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()?;

        Some(EventProducer {
            inner: &self.inner,
            _pd: PhantomData::<E>::default(),
        })
    }

    pub(crate) fn consumer(&self) -> Option<EventConsumer<'_, E, Q>> {
        self.consumer_split
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()?;

        Some(EventConsumer {
            inner: &self.inner,
            _pd: PhantomData::<E>::default(),
        })
    }
}

impl<E, Q: QueueAdapter<E> + MultiProducer> EventQueue<E, Q> {
    pub fn enqueue(&self, data: E) -> Result<(), E> {
        self.inner.enqueue(data)
    }
}

impl<E, Q: QueueAdapter<E>> EventProducer<'_, E, Q> {
    /// Enqueues an item to the underlying queue via the producer interface
    /// # Errors
    /// If the queue is unable to enqueue the data, it will return an error.
    pub fn enqueue(&mut self, event: E) -> Result<(), E> {
        self.inner.enqueue(event)
    }
}

impl<E, Q: QueueAdapter<E> + MultiProducer> Clone for EventProducer<'_, E, Q> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            _pd: PhantomData::<E>,
        }
    }
}

pub(crate) struct EventConsumer<'a, E, Q> {
    inner: &'a Q,
    _pd: PhantomData<E>,
}

impl<E, Q: QueueAdapter<E>> EventConsumer<'_, E, Q> {
    pub(crate) fn dequeue(&mut self) -> Option<E> {
        self.inner.dequeue()
    }
}

#[cfg(test)]
mod tests {
    use super::{EventQueue, MultiProducer, QueueAdapter};
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
    fn producer_split_only_succeeds_once() {
        let queue = EventQueue::<u32, _>::new(TestQueue::new());

        assert!(queue.producer().is_some());
        assert!(queue.producer().is_none());
    }

    #[test]
    fn consumer_split_only_succeeds_once() {
        let queue = EventQueue::<u32, _>::new(TestQueue::new());

        assert!(queue.consumer().is_some());
        assert!(queue.consumer().is_none());
    }

    #[test]
    fn cloned_producers_can_enqueue() {
        let queue = EventQueue::<u32, _>::new(TestQueue::new());
        let mut producer = queue.producer().expect("first producer call should succeed");
        let mut consumer = queue.consumer().expect("first consumer call should succeed");
        let mut producer_clone = producer.clone();

        assert_eq!(producer.enqueue(7), Ok(()));
        assert_eq!(producer_clone.enqueue(8), Err(8));
        assert_eq!(consumer.dequeue(), Some(7));
    }
}
