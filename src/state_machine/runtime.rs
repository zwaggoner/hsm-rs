use crate::state_machine::{Action, State, StateMachineDef, DEFAULT_MAX_NEST_DEPTH};
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
pub struct StateMachine<
    Sm: StateMachineDef + 'static,
    S: RunState = Init,
> {
    curr_state: Option<State<Sm>>,
    _pd: PhantomData<S>,
}

impl<Sm: StateMachineDef, S: RunState>
    StateMachine<Sm, S>
{
    fn exit_to(&mut self, context: &mut Sm, depth: usize) {
        while let Some(curr_state) = self.curr_state {
            if curr_state.depth > depth {
                (curr_state.exit)(context);

                self.curr_state = curr_state.parent;
            } else {
                return;
            }
        }
    }

    fn initial_entry_path(&mut self, target: State<Sm>) -> StatePath<Sm> {
        let mut entry_path: StatePath<Sm> = StatePath::<Sm>::new(); 
        let mut target_tree_state = target;
        let mut initial_path_found = false;

        entry_path.push(target_tree_state)
            .expect("Unexpectedly exceeded path capacity");

        while !initial_path_found {
            if let Some(curr_state) = self.curr_state {
                if target_tree_state.depth <= curr_state.depth + 1 {
                    initial_path_found = true;
                } else if target_tree_state.depth < curr_state.depth {
                    assert!(false, "Unexpected configuration");
                }
            }

            if let Some(target_tree_state_parent) = target_tree_state.parent {
                target_tree_state = target_tree_state_parent;
            }
            else {
                initial_path_found = true;
            }
        }

        entry_path
    }

    fn transition(&mut self, context: &mut Sm, target: State<Sm>) {
        //let mut child_initial_transition = false;
        let mut transition_target = Some(target);

        while let Some(mut target_state) = transition_target {
            self.exit_to(context, target_state.depth - 1);

            let mut entry_path = self.initial_entry_path(target_state);

            target_state = entry_path.last().expect("Unexpectedly empty entry path");

            while target_state.parent != self.curr_state {
                if let Some(curr_state) = self.curr_state {
                    assert!(target_state.depth == curr_state.depth || target_state.depth == curr_state.depth + 1, "Unexpected target_state and curr_state configuration");

                    (curr_state.exit)(context);
                    self.curr_state = curr_state.parent;

                    if target_state.depth != curr_state.depth + 1 {
                        continue;
                    }
                }

                entry_path.push(target_state).expect("Unexpectedly exceeded path capacity");

                if let Some(target_state_parent) = target_state.parent {
                    target_state = target_state_parent;
                } else {
                    break;
                }
            }

            entry_path.reverse();

            for state in entry_path.iter() {
                (state.entry)(context);
            }

            self.curr_state = entry_path.last().map(|v| &**v); 

            transition_target = (self.curr_state.unwrap().initial)(context);
        }
    }
}

impl<Sm: StateMachineDef> Default
    for StateMachine<Sm, Init>
{
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
        self.transition(context, target);

        StateMachine::<Sm, Run> {
            curr_state: self.curr_state,
            _pd: PhantomData::<Run>,
        }
    }
}

impl<Sm: StateMachineDef> StateMachine<Sm, Run> {
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
