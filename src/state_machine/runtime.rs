use crate::state_machine::{Action, State, StateMachineDef};
use crate::util::fixed_vec::FixedVec;
use core::marker::PhantomData;

mod _private {
    pub trait Sealed {}
}

/// Sealed trait marker for [`Run`] and [`Init`]
pub trait RunState: _private::Sealed {}

/// Marker for [`StateMachine`] indicating that it is in the `Init` phase.
pub struct Init {}

impl _private::Sealed for Init {}
impl RunState for Init {}

/// Marker for [`StateMachine`] indicating that it is in the `Run` phase.
pub struct Run {}

impl _private::Sealed for Run {}
impl RunState for Run {}

/// Runtime `StateMachine` object. Instatiates a state machine that can actually be used for
/// execution. The `Sm` (state machine) object implementing [`StateMachineDef`] must be supplied.
/// There is also an optional `MAX_NEST_DEPTH` which is defaulted to 8, and can be adjusted if the
/// user requires more deeply nested state machines, or reduced to reduce runtime footprint if
/// appropriate, and deeper state machines are not needed.
pub struct StateMachine<
    Sm: StateMachineDef + 'static,
    const MAX_NEST_DEPTH: usize = 8,
    S: RunState = Init,
> {
    curr_state: Option<State<Sm>>,
    path: FixedVec<State<Sm>, MAX_NEST_DEPTH>,
    _pd: PhantomData<S>,
}

impl<Sm: StateMachineDef, const MAX_NEST_DEPTH: usize, S: RunState>
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
                Ok(()) => (),
                Err(()) => excess_depth += 1,
            }

            curr_state = parent;
        }

        let depth = path.len() + excess_depth;

        assert!(
            depth <= MAX_NEST_DEPTH,
            "Path to state exceeds MAX_NEST_DEPTH: {MAX_NEST_DEPTH}, suggest increasing to {depth}"
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

    fn transition_recurse(&mut self, context: &mut Sm, source: Option<State<Sm>>, dest: State<Sm>, leaf: bool) {
        if let Some(mut source_state) = source {
            if source_state.depth >= dest.depth {
                while source_state.depth > dest.depth {
                    (source_state.exit)(context);

                    source_state = source_state.parent.expect("Unexpectedly null source parent");
                }

                if !(!leaf && source_state == dest) {
                    (source_state.exit)(context);
                }

                if source_state.parent != dest.parent {
                    self.transition_recurse(context, source_state.parent, dest.parent.expect("Unexpectedly null destination parent"), false);
                }
            }
            else if let Some(dest_parent) = dest.parent && dest.depth > source_state.depth {
                self.transition_recurse(context, Some(source_state), dest_parent, false);
            }
            else {
                assert!(true, "Unexpected state hierarchy");
            }
        }
        else {
            if let Some(parent) = dest.parent {
                self.transition_recurse(context, source, parent, false);
            }
        }

        (dest.entry)(context);

        if leaf {
            self.curr_state = Some(dest);

            if let Some(target) = (dest.initial)(context) {
                self.transition_recurse(context, self.curr_state, target, true);
            }
        }
    }

    fn transition(&mut self, context: &mut Sm, target: State<Sm>) {
        self.transition_recurse(context, self.curr_state, target, true);
        /*
        let mut child_initial_transition = false;
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

            if child_initial_transition {
                assert!(
                    enter_exit_target == self.path.len(),
                    "Initial transition targets must point to a new child state, detected differing parent tree in initial transition"
                );
            }

            // Exit to LCA
            self.exit_to(context, enter_exit_target);

            // Enter to leaf state
            self.enter(context, &target_path[enter_exit_target..]);

            // Check for initial transition in leaf state
            if let Some(leaf_state) = self.path.last() {
                transition_target = (leaf_state.initial)(context);

                if let Some(target) = transition_target {
                    child_initial_transition = true;
                    assert!(
                        !self.path.contains(&target),
                        "Initial transition targets must point to a new child state, detected initial transition to state already in state hierarchy"
                    );
                }
            }
        }
        */
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

impl<Sm: StateMachineDef, const MAX_NEST_DEPTH: usize> Default
    for StateMachine<Sm, MAX_NEST_DEPTH, Init>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Sm: StateMachineDef, const MAX_NEST_DEPTH: usize> StateMachine<Sm, MAX_NEST_DEPTH, Init> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            curr_state: None,
            path: FixedVec::<State<Sm>, MAX_NEST_DEPTH>::new(),
            _pd: PhantomData::<Init>,
        }
    }

    pub fn initial(mut self, context: &mut Sm) -> StateMachine<Sm, MAX_NEST_DEPTH, Run> {
        let target = <Sm as StateMachineDef>::initial(context);
        self.transition(context, target);

        StateMachine::<Sm, MAX_NEST_DEPTH, Run> {
            curr_state: self.curr_state,
            path: self.path,
            _pd: PhantomData::<Run>,
        }
    }
}

impl<Sm: StateMachineDef, const MAX_NEST_DEPTH: usize> StateMachine<Sm, MAX_NEST_DEPTH, Run> {
    pub fn dispatch(&mut self, context: &mut Sm, event: &Sm::Event) {
        let mut handled = Action::<Sm>::Unhandled;
        let mut state_opt = self.curr_state;

        while let Some(state) = state_opt {
            handled = (state.handler)(context, event);
            match handled {
                Action::<Sm>::Handled => break,
                Action::<Sm>::Transition(new_state) => {
                    self.transition(context, new_state);
                    break;
                }
                Action::Unhandled => (),
            }

            state_opt = state.parent;
        }

        assert!(!matches!(handled, Action::<Sm>::Unhandled), "Unhandled event detected");
    }
}
