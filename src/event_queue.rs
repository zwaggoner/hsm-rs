use crate::mpmc_bounded_queue::MpmcBoundedQueue;

pub struct EventProducer<'a, E, const SIZE: usize> {
    inner: &'a MpmcBoundedQueue::<E, SIZE>,
}

impl<'a, E, const SIZE: usize> EventProducer<'a, E, SIZE> {
    pub fn enqueue(&self, event: E) -> Result<(), ()> {
        self.inner.enqueue(event)
    }
}

impl<'a, E, const SIZE: usize> Clone for EventProducer<'a, E, SIZE> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
        }
    }
}

pub(crate) struct EventQueue<E, const SIZE: usize> {
    inner: MpmcBoundedQueue<E, SIZE>,
}

impl<E, const SIZE: usize> EventQueue<E, SIZE> {
    pub(crate) fn new() -> Self {
        Self {
            inner: MpmcBoundedQueue::<E, SIZE>::new(),
        }
    }

    pub(crate) fn dequeue(&self) -> Option<E> {
        self.inner.dequeue()
    }

    pub(crate) fn producer<'a>(&'a self) -> EventProducer<'a, E, SIZE> {
        EventProducer::<E, SIZE> {
            inner: &self.inner
        }
    }
}

