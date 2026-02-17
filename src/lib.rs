//#![no_std]

#[macro_export]
macro_rules! state {
    (
        impl $name:ident {
            $($body:tt)*
        }
    ) => {
        struct $name;

        impl $crate::HsmState for $name {
            $($body)*
        }
    };
    (runtime $s:ty) => {
        &<$s as $crate::RuntimeState>::STATE
    };
}

type _State<C, E> = &'static StateDesc<C, E>;
pub type State<S> = _State<<S as HsmState>::Context, <S as HsmState>::Event>;

pub enum Action<C: 'static + std::fmt::Debug, E: 'static + std::fmt::Debug> {
    Unhandled,
    Handled,
    Transition(_State<C, E>),
}

pub type StateAction<S> = Action<<S as HsmState>::Context, <S as HsmState>::Event>;

#[doc(hidden)]
#[derive(Debug)]
pub struct StateDesc<C: 'static + std::fmt::Debug, E: 'static + std::fmt::Debug> {
    type_id: core::any::TypeId,
    parent: Option<_State<C, E>>,
    initial: fn(&mut C) -> Option<_State<C, E>>,
    entry: fn(&mut C),
    handler: fn(&mut C, &E) -> Action<C, E>,
    exit: fn(&mut C),
}

impl<C: 'static + std::fmt::Debug, E: 'static + std::fmt::Debug> PartialEq for StateDesc<C, E> {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

pub trait HsmState {
    type Context: 'static + std::fmt::Debug;
    type Event: 'static + std::fmt::Debug;

    const PARENT: Option<State<Self>> = None;

    fn initial(_context: &mut Self::Context) -> Option<State<Self>> {
        None
    }

    fn entry(_context: &mut Self::Context) {}

    fn handler(_context: &mut Self::Context, _event: &Self::Event) -> StateAction<Self> {
        StateAction::<Self>::Unhandled
    }

    fn exit(_context: &mut Self::Context) {}
}

pub trait RuntimeState: HsmState {
    const STATE: StateDesc<Self::Context, Self::Event>;
}

impl<S: HsmState + 'static> RuntimeState for S {
    const STATE: StateDesc<S::Context, S::Event> = StateDesc::<S::Context, S::Event> {
        type_id: core::any::TypeId::of::<S>(),
        parent: S::PARENT,
        initial: S::initial,
        entry: S::entry,
        handler: S::handler,
        exit: S::exit,
    };
}

pub struct StateMachine<S: HsmState, const MAX_NEST_DEPTH: usize = 32> {
    path: [Option<State<S>>; MAX_NEST_DEPTH],
    curr_depth: usize,
}

impl<S: HsmState + 'static, const MAX_NEST_DEPTH: usize> StateMachine<S, MAX_NEST_DEPTH> {
    pub fn default() -> Self {
        Self {
            path: [None; MAX_NEST_DEPTH],
            curr_depth: 0,
        }
    }

    fn get_path(state: State<S>) -> (usize, [Option<State<S>>; MAX_NEST_DEPTH]) {
        let mut curr_state = state;
        let mut path: [Option<State<S>>; MAX_NEST_DEPTH] = [None; MAX_NEST_DEPTH];
        let mut depth = 0;

        path[depth] = Some(state);
        depth += 1;

        while let Some(parent) = curr_state.parent {
            if depth < MAX_NEST_DEPTH {
                path[depth] = Some(parent);
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

        path[..depth].reverse();

        (depth, path)
    }

    fn find_lca(
        &self,
        target_depth: usize,
        target_path: &[Option<State<S>>; MAX_NEST_DEPTH],
    ) -> Option<usize> {
        let max_search_depth = core::cmp::min(self.curr_depth, target_depth);

        // If the max depth of either tree is 0, there's no LCA
        if max_search_depth == 0 {
            return None;
        }

        // Same if top differ
        if self.path[0].unwrap() != target_path[0].unwrap() {
            return None;
        }

        let mut last_common_ancester = 0;

        for i in 1..max_search_depth {
            if self.path[i].unwrap() == target_path[i].unwrap() {
                last_common_ancester = i;
            } else {
                break;
            }
        }

        Some(last_common_ancester)
    }

    fn transition(&mut self, context: &mut S::Context, target: State<S>) {
        let mut transition_target = Some(target);

        while let Some(state) = transition_target {
            // Compute new tree
            let (target_depth, target_path) = Self::get_path(state);

            // Find LCA
            let enter_exit_target: usize =
                if let Some(lca) = self.find_lca(target_depth, &target_path) {
                    lca + 1
                } else {
                    0
                };

            // Exit to LCA
            self.exit_to(context, enter_exit_target);

            // Enter to leaf state
            self.enter_from(
                context,
                enter_exit_target,
                &target_path[enter_exit_target..target_depth],
            );

            // Check for initial transition in leaf state
            let leaf_state = self.path[self.curr_depth - 1].unwrap();

            transition_target = (leaf_state.initial)(context);
        }
    }

    fn enter_from(
        &mut self,
        context: &mut S::Context,
        start: usize,
        target_path: &[Option<State<S>>],
    ) {
        let mut depth = start;

        for state in target_path {
            self.path[depth] = *state;
            (state.unwrap().entry)(context);
            depth += 1;
        }

        self.curr_depth = depth;
    }

    fn exit_to(&mut self, context: &mut S::Context, end: usize) {
        for depth in (end..self.curr_depth).rev() {
            if let Some(state) = self.path[depth] {
                (state.exit)(context);
                self.path[depth] = None;

                if self.curr_depth > 0 {
                    self.curr_depth -= 1;
                } else {
                    break;
                }
            }
        }
    }

    pub fn dispatch(&mut self, context: &mut S::Context, event: &S::Event) {
        for depth in (0..self.curr_depth).rev() {
            if let Some(state) = self.path[depth] {
                match (state.handler)(context, event) {
                    StateAction::<S>::Handled => break,
                    StateAction::<S>::Transition(new_state) => {
                        self.transition(context, new_state);
                        break;
                    }
                    _ => continue,
                }
            }
        }
    }

    pub fn run(&mut self, context: &mut S::Context) {
        self.transition(context, state!(runtime S));
    }
}
