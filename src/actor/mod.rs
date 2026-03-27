/// The `queue` module provides some framework-compliant queue backends, as well
/// as adapter traits to be able to adapt your own queue or a RTOS queue.queue
pub mod queue;

///The `runtime` module provides some different scheduling disciplines for the actor run-to-completion steps.
pub mod runtime;

use self::queue::{EventConsumer, EventQueue, QueueAdapter};
use crate::state_machine::{Init, Run, StateMachine, StateMachineDef};

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
    'a,
    Sm: StateMachineDef + 'static,
    Q: QueueAdapter<Sm::Event>,
    const MAX_NEST_DEPTH: usize = 8,
> {
    context: Sm,
    sm: CurrSM<Sm, MAX_NEST_DEPTH>,
    event_consumer: EventConsumer<'a, Sm::Event, Q>,
    next_event: Option<Sm::Event>,
    initialized: bool,
}

impl<'a, Sm: StateMachineDef, Q: QueueAdapter<Sm::Event>, const MAX_NEST_DEPTH: usize>
    Actor<'a, Sm, Q, MAX_NEST_DEPTH>
{
    /// Constructs a new actor given its context object and underlying event queue
    ///
    /// # Panics
    /// The `EventQueue` is limited to a single consumer. If the consumer has already been taken for
    /// the queue by another actor, the constructor will panic.
    pub fn new(context: Sm, event_queue: &'a EventQueue<Sm::Event, Q>) -> Self {
        Self {
            context,
            sm: CurrSM::default(),
            event_consumer: event_queue
                .take_consumer()
                .expect("EventConsumer already taken"),
            next_event: None,
            initialized: false,
        }
    }

    fn prefetch_event(&mut self) -> bool {
        self.next_event = self.event_consumer.dequeue();
        self.next_event.is_some()
    }
}

impl<Sm: StateMachineDef, Q: QueueAdapter<Sm::Event>, const MAX_NEST_DEPTH: usize> ActorRuntime
    for Actor<'_, Sm, Q, MAX_NEST_DEPTH>
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
