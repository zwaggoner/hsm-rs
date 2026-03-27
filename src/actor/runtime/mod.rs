use crate::actor::ActorRuntime;

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

/// Superloop scheduler, gives each actor exactly one run-to-completion `step` per iteration of the
/// loop. Executes in order of actors passed to `new`
pub struct Superloop<'a, const NUM_ACTORS: usize> {
    inner: ToSchedule<'a, NUM_ACTORS>,
}

/// Cooperative scheduler, searches for the highest priority actor with work to be done,
/// reschedules. If higher priority actors are idle it moves onto the next available actor with
/// work to do. Priority is given by the order of actors passed to `new`
pub struct Cooperative<'a, const NUM_ACTORS: usize> {
    inner: ToSchedule<'a, NUM_ACTORS>,
}

impl<'a, const NUM_ACTORS: usize> Superloop<'a, NUM_ACTORS> {
    /// Construct a new Superloop scheduler. The order of `actors` is the order that the superloop
    /// executes the actors. The idle_task is run in the case where there were no actors that did
    /// work in the iteration of the superloop. 
    pub fn new(actors: [&'a mut dyn ActorRuntime; NUM_ACTORS], idle_task: Option<fn()>) -> Self {
        Self {
            inner: ToSchedule::new(actors, idle_task),
        }
    }

    /// Run method for the superloop scheduler. Note that this function never returns, once the
    /// actors are run they will execute in the context where you run this function. 
    pub fn run(&mut self) -> ! {
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
    /// Construct a new Cooperative scheduler. The order of `actors` is the priority order for the 
    /// scheduler. The idle_task is run in the case where there were no actors that did
    /// work in the iteration of the superloop. 
    pub fn new(actors: [&'a mut dyn ActorRuntime; NUM_ACTORS], idle_task: Option<fn()>) -> Self {
        Self {
            inner: ToSchedule::new(actors, idle_task),
        }
    }

    /// Run method for the cooperative scheduler. Note that this function never returns, once the
    /// actors are run they will execute in the context where you run this function. 
    pub fn run(&mut self) -> ! {
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

