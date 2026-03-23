use crate::event_queue::{EventConsumer, QueueAdapter};
use crate::state_machine::StateMachineSpec;
use crate::state_machine_runtime::{Init, Run, StateMachine};

pub trait ActorRuntime {
    fn initialized(&self) -> bool;
    fn step(&mut self) -> StepStatus;
}

pub enum StepStatus {
    Initialized { pending: bool },
    Ran { pending: bool },
    Idle,
}

impl StepStatus {
    pub fn is_idle(&self) -> bool {
        matches!(self, StepStatus::Idle)
    }

    pub fn is_pending(&self) -> bool {
        match self {
            StepStatus::Initialized { pending } | StepStatus::Ran { pending } => *pending,
            StepStatus::Idle => false,
        }
    }

    pub fn did_work(&self) -> bool {
        !self.is_idle()
    }
}

enum CurrSM<Sm: StateMachineSpec + 'static, const MAX_NEST_DEPTH: usize> {
    Init(StateMachine<Sm, MAX_NEST_DEPTH, Init>),
    Run(StateMachine<Sm, MAX_NEST_DEPTH, Run>),
}

impl<Sm: StateMachineSpec, const MAX_NEST_DEPTH: usize> Default for CurrSM<Sm, MAX_NEST_DEPTH> {
    fn default() -> Self {
        CurrSM::Init(StateMachine::<Sm, MAX_NEST_DEPTH>::default())
    }
}

pub struct Actor<
    'a,
    Sm: StateMachineSpec + 'static,
    Q: QueueAdapter<Sm::Event>,
    const MAX_NEST_DEPTH: usize = 8,
> {
    context: Sm,
    sm: CurrSM<Sm, MAX_NEST_DEPTH>,
    event_consumer: EventConsumer<'a, Sm::Event, Q>,
    next_event: Option<Sm::Event>,
    initialized: bool,
}

impl<'a, Sm: StateMachineSpec, Q: QueueAdapter<Sm::Event>, const MAX_NEST_DEPTH: usize>
    Actor<'a, Sm, Q, MAX_NEST_DEPTH>
{
    pub fn new(context: Sm, event_consumer: EventConsumer<'a, Sm::Event, Q>) -> Self {
        Self {
            context,
            sm: CurrSM::default(),
            event_consumer,
            next_event: None,
            initialized: false,
        }
    }
}

impl<Sm: StateMachineSpec, Q: QueueAdapter<Sm::Event>, const MAX_NEST_DEPTH: usize> ActorRuntime
    for Actor<'_, Sm, Q, MAX_NEST_DEPTH>
{
    fn initialized(&self) -> bool {
        self.initialized
    }

    fn step(&mut self) -> StepStatus {
        let mut step_status: StepStatus = StepStatus::Idle;

        if let CurrSM::Run(sm) = &mut self.sm {
            if let Some(event) = self.next_event.take() {
                sm.dispatch(&mut self.context, &event);
                step_status = StepStatus::Ran { pending: false };
            } else if let Some(event) = self.event_consumer.dequeue() {
                sm.dispatch(&mut self.context, &event);
                step_status = StepStatus::Ran { pending: false };
            }
        } else if let CurrSM::Init(sm) = core::mem::take(&mut self.sm) {
            self.sm = CurrSM::Run(sm.initial(&mut self.context));
            step_status = StepStatus::Initialized { pending: false };
            self.initialized = true;
        }

        match &mut step_status {
            StepStatus::Ran { pending } | StepStatus::Initialized { pending } => {
                if let Some(next_event) = self.event_consumer.dequeue() {
                    self.next_event = Some(next_event);
                    *pending = true;
                }
            },
            StepStatus::Idle => (),
        }

        step_status
    }
}
