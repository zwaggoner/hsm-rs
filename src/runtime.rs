use crate::actor::ActorRuntime;

pub trait Runtime {
    fn run(&mut self) -> !;
}

struct ToSchedule<'a, const NUM_ACTORS: usize> {
    actors: [&'a mut dyn ActorRuntime; NUM_ACTORS],
    idle_task: fn(),
}

impl<'a, const NUM_ACTORS: usize> ToSchedule<'a, NUM_ACTORS> {
    fn new(actors: [&'a mut dyn ActorRuntime; NUM_ACTORS], idle_task: Option<fn()>) -> Self {
        Self {
            actors,
            idle_task: {
                if let Some(idle_task) = idle_task {
                    idle_task
                } else {
                    || {}
                }
            },
        }
    }
}

pub struct Superloop<'a, const NUM_ACTORS: usize> {
    inner: ToSchedule<'a, NUM_ACTORS>,
}

pub struct Cooperative<'a, const NUM_ACTORS: usize> {
    inner: ToSchedule<'a, NUM_ACTORS>,
}

impl<'a, const NUM_ACTORS: usize> Superloop<'a, NUM_ACTORS> {
    pub fn new(actors: [&'a mut dyn ActorRuntime; NUM_ACTORS], idle_task: Option<fn()>) -> Self {
        Self {
            inner: ToSchedule::new(actors, idle_task),
        }
    }
}

impl<'a, const NUM_ACTORS: usize> Runtime for Superloop<'a, NUM_ACTORS> {
    fn run(&mut self) -> ! {
        loop {
            let mut ran: bool = false;

            for a in &mut self.inner.actors {
                ran |= a.step().did_work();
            }

            if !ran {
                (self.inner.idle_task)();
            }
        }
    }
}

impl<'a, const NUM_ACTORS: usize> Cooperative<'a, NUM_ACTORS> {
    pub fn new(actors: [&'a mut dyn ActorRuntime; NUM_ACTORS], idle_task: Option<fn()>) -> Self {
        Self {
            inner: ToSchedule::new(actors, idle_task),
        }
    }
}

impl<'a, const NUM_ACTORS: usize> Runtime for Cooperative<'a, NUM_ACTORS> {
    fn run(&mut self) -> ! {
        // For cooperative scheduler since runtime isn't guaranteed, initialize first
        for a in &mut self.inner.actors {
            if !a.initialized() {
                let _ = a.step();
            }
        }

        loop {
            let mut ran: bool = false;

            for a in &mut self.inner.actors {
                // If someone did work, reassess scheduling
                if a.step().did_work() {
                    ran = true;
                    break;
                }
            }

            // Only perform idle task if there was an iteration where nobody did work
            if !ran {
                (self.inner.idle_task)();
            }
        }
    }
}
