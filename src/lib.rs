#![no_std]

use core::mem::MaybeUninit;
use core::ops::Deref;
use core::ops::DerefMut;

pub trait Hsm {
    type Context: 'static;
    type Event: 'static;
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
    initial: fn(&mut H::Context) -> Option<State<H>>,
    entry: fn(&mut H::Context),
    handler: fn(&mut H::Context, &H::Event) -> Action<H>,
    exit: fn(&mut H::Context),
}

impl<H: Hsm> PartialEq for StateDesc<H> {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

pub trait HsmState<H: Hsm + 'static> {
    const PARENT: Option<State<H>> = None;

    fn initial(_context: &mut H::Context) -> Option<State<H>> {
        None
    }

    fn entry(_context: &mut H::Context) {}

    fn handler(_context: &mut H::Context, _event: &H::Event) -> Action<H> {
        Action::<H>::Unhandled
    }

    fn exit(_context: &mut H::Context) {}
}

pub trait RuntimeState<H: Hsm + 'static>: HsmState<H> {
    const STATE: StateDesc<H>;
}

impl<H: Hsm + 'static, S: HsmState<H> + 'static> RuntimeState<H> for S {
    const STATE: StateDesc<H> = StateDesc::<H> {
        type_id: core::any::TypeId::of::<S>(),
        parent: S::PARENT,
        initial: S::initial,
        entry: S::entry,
        handler: S::handler,
        exit: S::exit,
    };
}

struct FixedVec<T: Copy, const MAX_DEPTH: usize = 32> {
    arr: [MaybeUninit<T>; MAX_DEPTH],
    len: usize,
}

impl<T: Copy, const MAX_DEPTH: usize> FixedVec<T, MAX_DEPTH> {
    fn new() -> Self {
        FixedVec {
            arr: [const { MaybeUninit::uninit() }; MAX_DEPTH],
            len: 0,
        }
    }

    fn push(&mut self, elem: T) {
        assert!(self.len < MAX_DEPTH);
        self.arr[self.len].write(elem);
        self.len += 1;
    }

    fn pop(&mut self) -> Option<T> {
        if self.len > 0 {
            let elem = unsafe { self.arr[self.len - 1].assume_init() };

            self.len -= 1;

            Some(elem)
        } else {
            None
        }
    }
}

impl<T: Copy, const MAX_DEPTH: usize> Deref for FixedVec<T, MAX_DEPTH> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.arr.as_ptr() as *const T, self.len) }
    }
}

impl<T: Copy, const MAX_DEPTH: usize> DerefMut for FixedVec<T, MAX_DEPTH> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.arr.as_mut_ptr() as *mut T, self.len) }
    }
}

pub struct StateMachine<H: Hsm + 'static, const MAX_NEST_DEPTH: usize = 32> {
    path: FixedVec<State<H>, MAX_NEST_DEPTH>,
}

impl<H: Hsm, const MAX_NEST_DEPTH: usize> StateMachine<H, MAX_NEST_DEPTH> {
    pub fn new() -> Self {
        Self {
            path: FixedVec::<State<H>, MAX_NEST_DEPTH>::new(),
        }
    }

    fn get_path(state: State<H>) -> FixedVec<State<H>, MAX_NEST_DEPTH> {
        let mut curr_state = state;
        let mut path: FixedVec<State<H>, MAX_NEST_DEPTH> = FixedVec::new();
        let mut depth = 0;

        path.push(state);
        depth += 1;

        while let Some(parent) = curr_state.parent {
            if depth < MAX_NEST_DEPTH {
                path.push(parent);
            }

            depth += 1;
            curr_state = parent;
        }

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

        let mut last_common_ancester = 0;

        for i in 1..max_search_depth {
            if self.path[i] == target_path[i] {
                last_common_ancester = i;
            } else {
                break;
            }
        }

        Some(last_common_ancester)
    }

    fn transition(&mut self, context: &mut H::Context, target: State<H>) {
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

    fn enter(&mut self, context: &mut H::Context, target_path: &[State<H>]) {
        for state in target_path {
            self.path.push(state);
            (state.entry)(context);
        }
    }

    fn exit_to(&mut self, context: &mut H::Context, end: usize) {
        for _ in (end..self.path.len()).rev() {
            if let Some(state) = self.path.pop() {
                (state.exit)(context);
            }
        }
    }

    pub fn dispatch(&mut self, context: &mut H::Context, event: &H::Event) {
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

    pub fn run(&mut self, context: &mut H::Context, initial: State<H>) {
        self.transition(context, initial);
    }
}
