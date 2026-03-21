#![no_std]

mod event_queue;
mod fixed_vec;
mod mpmc_bounded_queue;

use core::marker::PhantomData;
use event_queue::EventQueue;
pub use event_queue::{EventProducer, QueueAdapter};
use fixed_vec::FixedVec;
pub use mpmc_bounded_queue::MpmcBoundedQueue;

pub trait Hsm: Sized {
    type Event: 'static;

    fn initial(&mut self) -> State<Self>;
}

pub type State<H> = &'static StateDesc<H>;

pub enum Action<H: Hsm + 'static> {
    Unhandled,
    Handled,
    Transition(State<H>),
}

#[doc(hidden)]
#[derive(Debug)]
pub struct StateDesc<H: Hsm + 'static> {
    type_id: core::any::TypeId,
    parent: Option<State<H>>,
    initial: fn(&mut H) -> Option<State<H>>,
    entry: fn(&mut H),
    handler: fn(&mut H, &H::Event) -> Action<H>,
    exit: fn(&mut H),
}

impl<H: Hsm> PartialEq for StateDesc<H> {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

mod _private {
    use super::{Hsm, StateDesc};

    pub trait Sealed {}
    pub trait RuntimeStateDesc<H: Hsm + 'static> {
        const STATE: StateDesc<H>;
    }
}

pub trait HsmState<S>: Hsm + Sized
where
    Self: 'static,
{
    type Parent: MaybeState<Self>;

    fn initial(&mut self) -> Option<State<Self>> {
        None
    }

    fn entry(&mut self) {}

    fn handler(&mut self, _event: &Self::Event) -> Action<Self> {
        Action::<Self>::Unhandled
    }

    fn exit(&mut self) {}
}

pub trait MaybeState<H: Hsm + 'static> : _private::Sealed {
    const OPT_STATE: Option<State<H>>;
}

pub struct AsState<S>(PhantomData<S>);
pub struct Top;

impl<S> _private::Sealed for AsState<S> { }
impl _private::Sealed for Top { }

impl<H: Hsm + 'static, S: 'static + _private::RuntimeStateDesc<H>> MaybeState<H> for AsState<S> {
    const OPT_STATE: Option<State<H>> = Some(&S::STATE);
}

impl<H: Hsm + 'static> MaybeState<H> for Top {
    const OPT_STATE: Option<State<H>> = None;
}

pub trait RuntimeState<H: Hsm + 'static> : _private::RuntimeStateDesc<H> {
    fn state() -> State<H>;
}

impl<S: 'static, H: HsmState<S> + 'static> _private::RuntimeStateDesc<H> for S {
    const STATE: StateDesc<H> = StateDesc::<H> {
        type_id: core::any::TypeId::of::<S>(),
        parent: H::Parent::OPT_STATE,
        initial: <H as HsmState<S>>::initial,
        entry: H::entry,
        handler: H::handler,
        exit: H::exit,
    };
}

impl<S: 'static + _private::RuntimeStateDesc<H>, H: Hsm + 'static> RuntimeState<H> for S {
    fn state() -> State<H> {
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

pub struct StateMachine<H: Hsm + 'static, const MAX_NEST_DEPTH: usize = 8, S: RunState = Init> {
    path: FixedVec<State<H>, MAX_NEST_DEPTH>,
    _pd: PhantomData<S>,
}

impl<H: Hsm, const MAX_NEST_DEPTH: usize, S: RunState> StateMachine<H, MAX_NEST_DEPTH, S> {
    fn get_path(state: State<H>) -> FixedVec<State<H>, MAX_NEST_DEPTH> {
        let mut curr_state = state;
        let mut path: FixedVec<State<H>, MAX_NEST_DEPTH> = FixedVec::new();
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

    fn find_lca(&self, target_path: &FixedVec<State<H>, MAX_NEST_DEPTH>) -> Option<usize> {
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

    fn transition(&mut self, context: &mut H, target: State<H>) {
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

    fn enter(&mut self, context: &mut H, target_path: &[State<H>]) {
        for state in target_path {
            self.path
                .push(state)
                .expect("Unexpectedly exceeded path capacity on entry");
            (state.entry)(context);
        }
    }

    fn exit_to(&mut self, context: &mut H, end: usize) {
        while self.path.len() > end {
            if let Some(state) = self.path.pop() {
                (state.exit)(context);
            }
        }
    }
}

impl<H: Hsm, const MAX_NEST_DEPTH: usize> Default for StateMachine<H, MAX_NEST_DEPTH, Init> {
    fn default() -> Self {
        Self::new()
    }
}

impl<H: Hsm, const MAX_NEST_DEPTH: usize> StateMachine<H, MAX_NEST_DEPTH, Init> {
    pub fn new() -> Self {
        Self {
            path: FixedVec::<State<H>, MAX_NEST_DEPTH>::new(),
            _pd: PhantomData::<Init>,
        }
    }

    pub fn initial(mut self, context: &mut H) -> StateMachine<H, MAX_NEST_DEPTH, Run> {
        let target = <H as Hsm>::initial(context);
        self.transition(context, target);

        StateMachine::<H, MAX_NEST_DEPTH, Run> {
            path: self.path,
            _pd: PhantomData::<Run>,
        }
    }
}

impl<H: Hsm, const MAX_NEST_DEPTH: usize> StateMachine<H, MAX_NEST_DEPTH, Run> {
    pub fn dispatch(&mut self, context: &mut H, event: &H::Event) {
        for state in self.path.iter().rev() {
            match (state.handler)(context, event) {
                Action::<H>::Handled => break,
                Action::<H>::Transition(new_state) => {
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

enum CurrSM<H: Hsm + 'static, const MAX_NEST_DEPTH: usize> {
    Init(StateMachine<H, MAX_NEST_DEPTH, Init>),
    Run(StateMachine<H, MAX_NEST_DEPTH, Run>),
}

impl<H: Hsm, const MAX_NEST_DEPTH: usize> Default for CurrSM<H, MAX_NEST_DEPTH> {
    fn default() -> Self {
        CurrSM::Init(StateMachine::<H, MAX_NEST_DEPTH>::default())
    }
}

pub struct Actor<H: Hsm + 'static, Q: QueueAdapter<H::Event>, const MAX_NEST_DEPTH: usize = 8> {
    context: H,
    sm: CurrSM<H, MAX_NEST_DEPTH>,
    event_queue: EventQueue<H::Event, Q>,
}

impl<H: Hsm, Q: QueueAdapter<H::Event>, const MAX_NEST_DEPTH: usize> Actor<H, Q, MAX_NEST_DEPTH> {
    pub fn new(context: H, queue: Q) -> Self {
        Self {
            context,
            sm: CurrSM::default(),
            event_queue: EventQueue::<H::Event, Q>::new(queue),
        }
    }

    pub fn event_producer<'a>(&'a self) -> EventProducer<'a, H::Event, Q> {
        self.event_queue.producer()
    }
}

impl<H: Hsm, Q: QueueAdapter<H::Event>, const MAX_NEST_DEPTH: usize> Step
    for Actor<H, Q, MAX_NEST_DEPTH>
{
    fn step(&mut self) -> bool {
        if let CurrSM::Run(sm) = &mut self.sm {
            if let Some(event) = self.event_queue.dequeue() {
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
