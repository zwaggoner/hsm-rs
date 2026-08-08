use crate::state_machine::{Action, DEFAULT_MAX_NEST_DEPTH, State, StateMachineDef};
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

type StatePath<Sm> = FixedVec<State<Sm>, DEFAULT_MAX_NEST_DEPTH>;

/// Runtime `StateMachine` object. Instatiates a state machine that can actually be used for
/// execution. The `Sm` (state machine) object implementing [`StateMachineDef`] must be supplied.
pub struct StateMachine<Sm: StateMachineDef + 'static, S: RunState = Init> {
    curr_state: Option<State<Sm>>,
    _pd: PhantomData<S>,
}

impl<Sm: StateMachineDef, S: RunState> StateMachine<Sm, S> {
    fn exit_to(&mut self, context: &mut Sm, target: State<Sm>) -> bool {
        let mut exited: bool = false;

        while let Some(curr_state) = self.curr_state {
            if curr_state.depth > target.depth {
                (curr_state.exit)(context);
                exited = true;
                self.curr_state = curr_state.parent;
            } else {
                break;
            }
        }

        exited
    }

    fn get_path_to_target(&self, target: State<Sm>) -> StatePath<Sm> {
        let depth: usize = self.curr_state.map_or(0, |state| state.depth);

        let mut entry_path: StatePath<Sm> = StatePath::<Sm>::new();

        if let Some(mut curr_target_path_state) = target.parent {
            while curr_target_path_state.depth > depth {
                entry_path
                    .push(curr_target_path_state)
                    .expect("Unexpectedly exceeded path capacity");

                if let Some(curr_target_path_state_parent) = curr_target_path_state.parent {
                    curr_target_path_state = curr_target_path_state_parent;
                } else {
                    break;
                }
            }
        }

        entry_path
    }

    fn get_entry_path_and_exit_to_lca(
        &mut self,
        context: &mut Sm,
        source: Option<State<Sm>>,
        target: State<Sm>,
        entry_path: &mut StatePath<Sm>,
    ) {
        let source_depth: usize = source.map_or(0, |state| state.depth);

        let mut curr_target_path_state = entry_path.last().map_or(target, |state| *state);

        while curr_target_path_state.parent != self.curr_state
            || curr_target_path_state.depth > (source_depth + 1)
        {
            if let Some(curr_state) = self.curr_state {
                (curr_state.exit)(context);
                self.curr_state = curr_state.parent;

                if curr_target_path_state.depth != curr_state.depth + 1 {
                    continue;
                }
            }

            if let Some(curr_target_path_state_parent) = curr_target_path_state.parent {
                entry_path
                    .push(curr_target_path_state_parent)
                    .expect("Unexpectedly exceeded path capacity");

                curr_target_path_state = curr_target_path_state_parent;
            } else {
                break;
            }
        }
    }

    fn transition(&mut self, context: &mut Sm, source: Option<State<Sm>>, target: State<Sm>) {
        let mut child_initial_transition = false;
        let mut transition_target = Some(target);
        let mut transition_source = source;

        while let Some(target_state) = transition_target {
            let curr_state_is_leaf = !self.exit_to(context, target_state);
            let mut entry_path: StatePath<Sm> = self.get_path_to_target(target_state);
            let mut enter_target: bool = true;

            if let Some(curr_state) = self.curr_state
                && curr_state == target_state
            {
                if curr_state_is_leaf {
                    (curr_state.exit)(context);
                } else {
                    enter_target = false;
                }
            } else {
                self.get_entry_path_and_exit_to_lca(
                    context,
                    transition_source,
                    target_state,
                    &mut entry_path,
                );
            }

            for state in entry_path.iter().rev() {
                (state.entry)(context);
            }

            if enter_target {
                (target_state.entry)(context);
            }

            transition_target = (target_state.initial)(context);
            transition_source = Some(target_state);

            self.curr_state = Some(target_state);

            if let Some(tt) = transition_target {
                assert!(
                    tt.depth > target_state.depth,
                    "Initial transitions must be to a valid child state"
                );
            }
        }
    }
}

impl<Sm: StateMachineDef> Default for StateMachine<Sm, Init> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Sm: StateMachineDef> StateMachine<Sm, Init> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            curr_state: None,
            _pd: PhantomData::<Init>,
        }
    }

    pub fn initial(mut self, context: &mut Sm) -> StateMachine<Sm, Run> {
        let target = <Sm as StateMachineDef>::initial(context);
        self.transition(context, None, target);

        StateMachine::<Sm, Run> {
            curr_state: self.curr_state,
            _pd: PhantomData::<Run>,
        }
    }
}

impl<Sm: StateMachineDef> StateMachine<Sm, Run> {
    /// Dispatches an event to the `StateMachine`
    ///
    /// # Panics
    /// All events must be handled, if they are not it is considered a hard fault. This is
    /// implemented so that safety-critical systems properly fault when unhandled events occur.
    /// Should you not have the same requirement, simply add a single topmost state encompassing the
    /// entire state machine, and add a "Handled" action for the handler for the topmost state.
    pub fn dispatch(&mut self, context: &mut Sm, event: &Sm::Event) {
        let mut handled = Action::<Sm>::Unhandled;
        let mut state_opt = self.curr_state;

        while let Some(state) = state_opt {
            handled = (state.handler)(context, event);
            match handled {
                Action::<Sm>::Handled => break,
                Action::<Sm>::Transition(new_state) => {
                    self.transition(context, Some(state), new_state);
                    break;
                }
                Action::Unhandled => (),
            }

            state_opt = state.parent;
        }

        assert!(
            !matches!(handled, Action::<Sm>::Unhandled),
            "Unhandled event detected"
        );
    }
}
