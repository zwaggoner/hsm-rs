#![no_std]

mod event_queue;
mod fixed_vec;
mod mpmc_bounded_queue;

use core::marker::PhantomData;
pub use event_queue::{EventConsumer, EventProducer, Mailbox, QueueAdapter};
use fixed_vec::FixedVec;
pub use mpmc_bounded_queue::MpmcBoundedQueue;

pub trait StateMachineSpec: Sized {
    type Event: 'static;

    fn initial(&mut self) -> State<Self>;
}

pub type State<Sm> = &'static StateDesc<Sm>;

pub enum Action<Sm: StateMachineSpec + 'static> {
    Unhandled,
    Handled,
    Transition(State<Sm>),
}

#[doc(hidden)]
#[derive(Debug)]
pub struct StateDesc<Sm: StateMachineSpec + 'static> {
    id: core::any::TypeId,
    parent: Option<State<Sm>>,
    initial: fn(&mut Sm) -> Option<State<Sm>>,
    entry: fn(&mut Sm),
    handler: fn(&mut Sm, &Sm::Event) -> Action<Sm>,
    exit: fn(&mut Sm),
}

impl<Sm: StateMachineSpec> PartialEq for StateDesc<Sm> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

mod _private {
    use super::{StateDesc, StateMachineSpec};

    pub trait Sealed {}
    pub trait StaticStateDesc<Sm: StateMachineSpec + 'static> {
        const STATE: StateDesc<Sm>;
    }
}

pub trait StateImpl<S>: StateMachineSpec + Sized
where
    Self: 'static,
{
    type Parent: ParentState<Self>;

    fn initial(&mut self) -> Option<State<Self>> {
        None
    }

    fn entry(&mut self) {}

    fn handler(&mut self, _event: &Self::Event) -> Action<Self> {
        Action::Unhandled
    }

    fn exit(&mut self) {}
}

pub trait ParentState<Sm: StateMachineSpec + 'static>: _private::Sealed {
    const OPT_STATE: Option<State<Sm>>;
}

pub struct Parent<S>(PhantomData<S>);
pub struct Root;

impl<S> _private::Sealed for Parent<S> {}
impl _private::Sealed for Root {}

impl<Sm: StateMachineSpec + 'static, S: 'static + _private::StaticStateDesc<Sm>> ParentState<Sm>
    for Parent<S>
{
    const OPT_STATE: Option<State<Sm>> = Some(&S::STATE);
}

impl<Sm: StateMachineSpec + 'static> ParentState<Sm> for Root {
    const OPT_STATE: Option<State<Sm>> = None;
}

pub trait StateRef<Sm: StateMachineSpec + 'static>: _private::StaticStateDesc<Sm> {
    fn state() -> State<Sm>;
}

impl<S: 'static, Sm: StateImpl<S> + 'static> _private::StaticStateDesc<Sm> for S {
    const STATE: StateDesc<Sm> = StateDesc::<Sm> {
        id: core::any::TypeId::of::<(Sm, S)>(),
        parent: Sm::Parent::OPT_STATE,
        initial: <Sm as StateImpl<S>>::initial,
        entry: Sm::entry,
        handler: Sm::handler,
        exit: Sm::exit,
    };
}

impl<S: 'static + _private::StaticStateDesc<Sm>, Sm: StateMachineSpec + 'static> StateRef<Sm>
    for S
{
    fn state() -> State<Sm> {
        &Self::STATE
    }
}

pub trait RunState: _private::Sealed {}

pub struct Init {}

impl _private::Sealed for Init {}
impl RunState for Init {}

pub struct Run {}

impl _private::Sealed for Run {}
impl RunState for Run {}

pub struct StateMachine<
    Sm: StateMachineSpec + 'static,
    const MAX_NEST_DEPTH: usize = 8,
    S: RunState = Init,
> {
    path: FixedVec<State<Sm>, MAX_NEST_DEPTH>,
    _pd: PhantomData<S>,
}

impl<Sm: StateMachineSpec, const MAX_NEST_DEPTH: usize, S: RunState>
    StateMachine<Sm, MAX_NEST_DEPTH, S>
{
    fn get_path(state: State<Sm>) -> FixedVec<State<Sm>, MAX_NEST_DEPTH> {
        let mut curr_state = state;
        let mut path: FixedVec<State<Sm>, MAX_NEST_DEPTH> = FixedVec::new();
        let mut excess_depth = 0;

        path.push(state)
            .expect("Unexpectedly exceeded capacity on first push to path");

        while let Some(parent) = curr_state.parent {
            match path.push(parent) {
                Ok(_) => (),
                Err(_) => excess_depth += 1,
            }

            curr_state = parent;
        }

        let depth = path.len() + excess_depth;

        assert!(
            depth <= MAX_NEST_DEPTH,
            "Path to state exceeds MAX_NEST_DEPTH: {}, suggest increasing to {}",
            MAX_NEST_DEPTH,
            depth
        );

        path.reverse();

        path
    }

    fn find_lca(&self, target_path: &FixedVec<State<Sm>, MAX_NEST_DEPTH>) -> Option<usize> {
        let max_search_depth = core::cmp::min(self.path.len(), target_path.len());

        // If the max depth of either tree is 0, there's no LCA
        if max_search_depth == 0 {
            return None;
        }

        // Same if top differ
        if self.path.first() != target_path.first() {
            return None;
        }

        let mut last_common_ancestor = 0;

        for i in 1..max_search_depth {
            if self.path[i] == target_path[i] {
                last_common_ancestor = i;
            } else {
                break;
            }
        }

        Some(last_common_ancestor)
    }

    fn transition(&mut self, context: &mut Sm, target: State<Sm>) {
        let mut transition_target = Some(target);

        while let Some(state) = transition_target {
            // Compute new tree
            let target_path = Self::get_path(state);

            // Find LCA
            let enter_exit_target: usize = if let Some(lca) = self.find_lca(&target_path) {
                lca + 1
            } else {
                0
            };

            // Exit to LCA
            self.exit_to(context, enter_exit_target);

            // Enter to leaf state
            self.enter(context, &target_path[enter_exit_target..]);

            // Check for initial transition in leaf state
            if let Some(leaf_state) = self.path.last() {
                transition_target = (leaf_state.initial)(context);
            }
        }
    }

    fn enter(&mut self, context: &mut Sm, target_path: &[State<Sm>]) {
        for state in target_path {
            self.path
                .push(state)
                .expect("Unexpectedly exceeded path capacity on entry");
            (state.entry)(context);
        }
    }

    fn exit_to(&mut self, context: &mut Sm, end: usize) {
        while self.path.len() > end {
            if let Some(state) = self.path.pop() {
                (state.exit)(context);
            }
        }
    }
}

impl<Sm: StateMachineSpec, const MAX_NEST_DEPTH: usize> Default
    for StateMachine<Sm, MAX_NEST_DEPTH, Init>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Sm: StateMachineSpec, const MAX_NEST_DEPTH: usize> StateMachine<Sm, MAX_NEST_DEPTH, Init> {
    pub fn new() -> Self {
        Self {
            path: FixedVec::<State<Sm>, MAX_NEST_DEPTH>::new(),
            _pd: PhantomData::<Init>,
        }
    }

    pub fn initial(mut self, context: &mut Sm) -> StateMachine<Sm, MAX_NEST_DEPTH, Run> {
        let target = <Sm as StateMachineSpec>::initial(context);
        self.transition(context, target);

        StateMachine::<Sm, MAX_NEST_DEPTH, Run> {
            path: self.path,
            _pd: PhantomData::<Run>,
        }
    }
}

impl<Sm: StateMachineSpec, const MAX_NEST_DEPTH: usize> StateMachine<Sm, MAX_NEST_DEPTH, Run> {
    pub fn dispatch(&mut self, context: &mut Sm, event: &Sm::Event) {
        for state in self.path.iter().rev() {
            match (state.handler)(context, event) {
                Action::<Sm>::Handled => break,
                Action::<Sm>::Transition(new_state) => {
                    self.transition(context, new_state);
                    break;
                }
                _ => continue,
            }
        }
    }
}

pub trait Step {
    fn step(&mut self) -> bool;
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
}

impl<'a, Sm: StateMachineSpec, Q: QueueAdapter<Sm::Event>, const MAX_NEST_DEPTH: usize>
    Actor<'a, Sm, Q, MAX_NEST_DEPTH>
{
    pub fn new(context: Sm, event_consumer: EventConsumer<'a, Sm::Event, Q>) -> Self {
        Self {
            context,
            sm: CurrSM::default(),
            event_consumer,
        }
    }
}

impl<'a, Sm: StateMachineSpec, Q: QueueAdapter<Sm::Event>, const MAX_NEST_DEPTH: usize> Step
    for Actor<'a, Sm, Q, MAX_NEST_DEPTH>
{
    fn step(&mut self) -> bool {
        if let CurrSM::Run(sm) = &mut self.sm {
            if let Some(event) = self.event_consumer.dequeue() {
                sm.dispatch(&mut self.context, &event);
                return true;
            }
        } else if let CurrSM::Init(sm) = core::mem::take(&mut self.sm) {
            self.sm = CurrSM::Run(sm.initial(&mut self.context));
            return true;
        }

        false
    }
}

pub trait Runtime {
    fn run(&mut self) -> !; 
}

pub struct Scheduler { }

pub struct Superloop<'a, const NUM_ACTORS: usize> {
    actors: [&'a mut dyn Step; NUM_ACTORS],
    idle_task: fn()
}

impl Scheduler {
    pub fn superloop<'a, const NUM_ACTORS: usize>(actors: [&'a mut dyn Step; NUM_ACTORS], idle_task: Option<fn()>) -> Superloop<'a, NUM_ACTORS> {
        Superloop {
            actors,
            idle_task: {
                if let Some(idle_task) = idle_task {
                    idle_task
                } else {
                    || {}
                }
            }
        }
    }
}

impl<'a, const NUM_ACTORS: usize> Runtime for Superloop<'a, NUM_ACTORS> {
    fn run(&mut self) -> ! {
        loop {
            let mut ran: bool = false;

            for a in &mut self.actors {
                ran &= a.step();
            }

            if !ran {
                (self.idle_task)();
            }
        }
    }
}
