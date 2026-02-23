//#![no_std]

pub trait Hsm {
    type Context: 'static + std::fmt::Debug;
    type Event: 'static + std::fmt::Debug;
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

pub struct StateMachine<H: Hsm + 'static, const MAX_NEST_DEPTH: usize = 32> {
    path: [Option<State<H>>; MAX_NEST_DEPTH],
    curr_depth: usize,
}

impl<H: Hsm, const MAX_NEST_DEPTH: usize> StateMachine<H, MAX_NEST_DEPTH> {
    pub fn default() -> Self {
        Self {
            path: [None; MAX_NEST_DEPTH],
            curr_depth: 0,
        }
    }

    fn get_path(state: State<H>) -> (usize, [Option<State<H>>; MAX_NEST_DEPTH]) {
        let mut curr_state = state;
        let mut path: [Option<State<H>>; MAX_NEST_DEPTH] = [None; MAX_NEST_DEPTH];
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
        target_path: &[Option<State<H>>; MAX_NEST_DEPTH],
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

    fn transition(&mut self, context: &mut H::Context, target: State<H>) {
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
        context: &mut H::Context,
        start: usize,
        target_path: &[Option<State<H>>],
    ) {
        let mut depth = start;

        for state in target_path {
            self.path[depth] = *state;
            (state.unwrap().entry)(context);
            depth += 1;
        }

        self.curr_depth = depth;
    }

    fn exit_to(&mut self, context: &mut H::Context, end: usize) {
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

    pub fn dispatch(&mut self, context: &mut H::Context, event: &H::Event) {
        for depth in (0..self.curr_depth).rev() {
            if let Some(state) = self.path[depth] {
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

    pub fn run(&mut self, context: &mut H::Context, initial: State<H>) {
        self.transition(context, initial);
    }
}
