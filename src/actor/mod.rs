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
    Initialized { pending: bool },

    /// Indicates that the actor dispatched an event on the call to `step`. The `pending` field
    /// indicates if there is an event pending.
    Ran { pending: bool },

    /// Indicates that there was no work done and no events to dispatch on the call to `step`.
    Idle,
}

impl StepStatus {
    /// Function returning if the `StepStatus` is `Idle`
    #[must_use]
    pub fn is_idle(&self) -> bool {
        matches!(self, StepStatus::Idle)
    }

    /// Function returning if `StepStatus` indicates that there is a pending event
    #[must_use]
    pub fn is_pending(&self) -> bool {
        match self {
            StepStatus::Initialized { pending } | StepStatus::Ran { pending } => *pending,
            StepStatus::Idle => false,
        }
    }

    /// Function returning if `StepStatus` indicates that there was work done
    #[must_use]
    pub fn did_work(&self) -> bool {
        !self.is_idle()
    }
}

enum CurrSM<Sm: StateMachineDef + 'static, const MAX_NEST_DEPTH: usize> {
    Init(StateMachine<Sm, MAX_NEST_DEPTH, Init>),
    Run(StateMachine<Sm, MAX_NEST_DEPTH, Run>),
}

impl<Sm: StateMachineDef, const MAX_NEST_DEPTH: usize> Default for CurrSM<Sm, MAX_NEST_DEPTH> {
    fn default() -> Self {
        CurrSM::Init(StateMachine::<Sm, MAX_NEST_DEPTH>::default())
    }
}

/// Actor object housing the underlying state machine, context object and event queue for
/// orchestrating actor behavior.
pub struct Actor<
    Sm: StateMachineDef + 'static,
    Q: QueueAdapter<Sm::Event>,
    const MAX_NEST_DEPTH: usize = 8,
> {
    event_queue: EventQueue<Sm::Event, Q>,
    _pdsm: PhantomData<CurrSM<Sm, MAX_NEST_DEPTH>>,
}

pub struct RuntimeActor<
    'a,
    Sm: StateMachineDef + 'static,
    Q: QueueAdapter<Sm::Event>,
    const MAX_NEST_DEPTH: usize,
> {
    event_consumer: EventConsumer<'a, Sm::Event, Q>,
    context: Sm,
    sm: CurrSM<Sm, MAX_NEST_DEPTH>,
    next_event: Option<Sm::Event>,
    initialized: bool,
}

impl<Sm: StateMachineDef, Q: QueueAdapter<Sm::Event>, const MAX_NEST_DEPTH: usize>
    Actor<Sm, Q, MAX_NEST_DEPTH>
{
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
    pub fn bind(&self, context: Sm) -> RuntimeActor<'_, Sm, Q, MAX_NEST_DEPTH> {
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

impl<Sm: StateMachineDef, Q: QueueAdapter<Sm::Event> + MultiProducer, const MAX_NEST_DEPTH: usize>
    Actor<Sm, Q, MAX_NEST_DEPTH>
{
    /// Enqueues an item to the underlying queue directly on the actor's event queue. This method is
    /// only available if the underlying queue is [`MultiProducer`]
    /// # Errors
    /// If the queue is unable to enqueue the data, it will return an error.
    pub fn enqueue(&self, data: Sm::Event) -> Result<(), Sm::Event> {
        self.event_queue.enqueue(data)
    }
}

impl<Sm: StateMachineDef, Q: QueueAdapter<Sm::Event>, const MAX_NEST_DEPTH: usize>
    RuntimeActor<'_, Sm, Q, MAX_NEST_DEPTH>
{
    fn prefetch_event(&mut self) -> bool {
        self.next_event = self.event_consumer.dequeue();
        self.next_event.is_some()
    }
}

impl<Sm: StateMachineDef, Q: QueueAdapter<Sm::Event>, const MAX_NEST_DEPTH: usize> ActorRuntime
    for RuntimeActor<'_, Sm, Q, MAX_NEST_DEPTH>
{
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
