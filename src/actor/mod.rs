/// The `queue` module provides some framework-compliant queue backends, as well
/// as adapter traits to be able to adapt your own queue or a RTOS queue.queue
pub mod queue;

///The `runtime` module provides some different scheduling disciplines for the actor run-to-completion steps.
pub mod runtime;

use self::queue::{EventConsumer, EventProducer, EventQueue, MultiProducer, QueueAdapter};
use crate::state_machine::{Init, Run, StateMachine, StateMachineDef};

use core::marker::PhantomData;

/// Traits required to schedule an actor
pub trait ActorRuntime {
    /// Indicates if actor is initialized after any potential initialization actions. This is only
    /// relevant for schedulers that treat initialization as different from runtime scheduling.
    fn initialized(&self) -> bool;

    /// Implements the run-to-completion step for the actor
    fn step(&mut self) -> StepStatus;
}

/// `enum` indicating the result of the run to completion step
pub enum StepStatus {
    /// Indicates that the actor was initialized on the call to `step`. The `pending` field
    /// indicates if there is an event pending.
    Initialized {
        /// True if events are pending, false if there are no events to dispatch
        pending: bool,
    },

    /// Indicates that the actor dispatched an event on the call to `step`. The `pending` field
    /// indicates if there is an event pending.
    Ran {
        /// True if events are pending, false if there are no events to dispatch
        pending: bool,
    },

    /// Indicates that there was no work done and no events to dispatch on the call to `step`.
    Idle,
}

impl StepStatus {
    /// Function returning if the `StepStatus` is `Idle`
    #[must_use]
    pub const fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    /// Function returning if `StepStatus` indicates that there is a pending event
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        match self {
            Self::Initialized { pending } | Self::Ran { pending } => *pending,
            Self::Idle => false,
        }
    }

    /// Function returning if `StepStatus` indicates that there was work done
    #[must_use]
    pub const fn did_work(&self) -> bool {
        !self.is_idle()
    }
}

enum CurrSM<Sm: StateMachineDef + 'static> {
    Init(StateMachine<Sm, Init>),
    Run(StateMachine<Sm, Run>),
}

impl<Sm: StateMachineDef> Default for CurrSM<Sm> {
    fn default() -> Self {
        Self::Init(StateMachine::<Sm>::default())
    }
}

/// Initial Actor object housing the `event_queue`.  
pub struct Actor<Sm: StateMachineDef + 'static, Q: QueueAdapter<Sm::Event>> {
    event_queue: EventQueue<Sm::Event, Q>,
    _pdsm: PhantomData<CurrSM<Sm>>,
}

/// Runtime Actor object housing the underlying state machine, context object and event consumer,
/// constructed from a call to `bind` on `Actor`.
pub struct RuntimeActor<'a, Sm: StateMachineDef + 'static, Q: QueueAdapter<Sm::Event>> {
    event_consumer: EventConsumer<'a, Sm::Event, Q>,
    context: Sm,
    sm: CurrSM<Sm>,
    next_event: Option<Sm::Event>,
    initialized: bool,
}

impl<Sm: StateMachineDef, Q: QueueAdapter<Sm::Event>> Actor<Sm, Q> {
    /// Constructs a new actor given its context object and underlying event queue
    pub const fn new(queue: Q) -> Self {
        Self {
            event_queue: EventQueue::new(queue),
            _pdsm: PhantomData,
        }
    }

    /// Takes the event producer for the actor. Can only be taken once, if the underlying queue is
    /// [`MultiProducer`], the [`EventProducer`] object can be cloned.
    pub fn take_producer(&self) -> Option<EventProducer<'_, Sm::Event, Q>> {
        self.event_queue.take_producer()
    }

    /// Binds the actor to its context object and produces the runtime actor.
    ///
    /// # Panics
    /// The Actor can only be bound once. If the actor has already been bound, the bind function
    /// will panic, as this represents a configuration error in the framework, not a resolvable
    /// runtime error.
    pub fn bind(&self, context: Sm) -> RuntimeActor<'_, Sm, Q> {
        RuntimeActor {
            event_consumer: self
                .event_queue
                .take_consumer()
                .expect("Actor has already been bound."),
            context,
            sm: CurrSM::default(),
            next_event: None,
            initialized: false,
        }
    }
}

impl<Sm: StateMachineDef, Q: QueueAdapter<Sm::Event> + MultiProducer> Actor<Sm, Q> {
    /// Enqueues an item to the underlying queue directly on the actor's event queue. This method is
    /// only available if the underlying queue is [`MultiProducer`]
    /// # Errors
    /// If the queue is unable to enqueue the data, it will return an error.
    pub fn enqueue(&self, data: Sm::Event) -> Result<(), Sm::Event> {
        self.event_queue.enqueue(data)
    }
}

impl<Sm: StateMachineDef, Q: QueueAdapter<Sm::Event>> RuntimeActor<'_, Sm, Q> {
    fn prefetch_event(&mut self) -> bool {
        self.next_event = self.event_consumer.dequeue();
        self.next_event.is_some()
    }
}

impl<Sm: StateMachineDef, Q: QueueAdapter<Sm::Event>> ActorRuntime for RuntimeActor<'_, Sm, Q> {
    fn initialized(&self) -> bool {
        self.initialized
    }

    fn step(&mut self) -> StepStatus {
        if let CurrSM::Run(sm) = &mut self.sm {
            if let Some(event) = self
                .next_event
                .take()
                .or_else(|| self.event_consumer.dequeue())
            {
                sm.dispatch(&mut self.context, &event);

                return StepStatus::Ran {
                    pending: self.prefetch_event(),
                };
            }
        } else if let CurrSM::Init(sm) = core::mem::take(&mut self.sm) {
            self.sm = CurrSM::Run(sm.initial(&mut self.context));
            self.initialized = true;
            return StepStatus::Initialized {
                pending: self.prefetch_event(),
            };
        }

        StepStatus::Idle
    }
}
