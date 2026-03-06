use core::marker::PhantomData;

pub trait QueueAdapter<T>: Default {
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
            _pd: PhantomData::<E>::default(),
        }
    }
}

pub(crate) struct EventQueue<E, Q: QueueAdapter<E>> {
    inner: Q,
    _pd: PhantomData<E>,
}

impl<E, Q: QueueAdapter<E>> EventQueue<E, Q> {
    pub(crate) fn new() -> Self {
        Self {
            inner: Q::default(),
            _pd: PhantomData::<E>::default(),
        }
    }

    pub(crate) fn dequeue(&self) -> Option<E> {
        self.inner.dequeue()
    }

    pub(crate) fn producer<'a>(&'a self) -> EventProducer<'a, E, Q> {
        EventProducer::<E, Q> {
            inner: &self.inner,
            _pd: PhantomData::<E>::default(),
        }
    }
}
