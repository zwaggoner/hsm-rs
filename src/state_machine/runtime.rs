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
    fn transition(&mut self, context: &mut Sm, target: State<Sm>) {
        //let mut child_initial_transition = false;
        let mut transition_target = Some(target);

        while let Some(mut target_state) = transition_target {
            let mut entry_path: StatePath<Sm> = StatePath::<Sm>::new();
            entry_path.push(target_state).expect("Unexpectedly exceeded path capacity");

            if let Some(mut curr_state) = self.curr_state {
                while curr_state.depth > target_state.depth {
                    (curr_state.exit)(context);
                    
                    if let Some(curr_state_parent) = curr_state.parent {
                        curr_state = curr_state_parent;
                    } else {
                        break;
                    }
                }

                if curr_state.depth == target_state.depth {
                    while curr_state.parent != target_state.parent {
                        (curr_state.exit)(context);

                        if let Some(curr_state_parent) = curr_state.parent {
                            curr_state = curr_state_parent;
                        } 

                        if let Some(target_state_parent) = target_state.parent {
                            entry_path.push(target_state_parent).expect("Unexpectedly exceeded path capacity");
                            target_state = target_state_parent;
                        }
                    }

                    (curr_state.exit)(context);
                }
                else {
                    while target_state.depth > curr_state.depth {
                        if let Some(target_state_parent) = target_state.parent {
                            if target_state_parent != curr_state {
                                entry_path.push(target_state_parent).expect("Unexpectedly exceeded path capacity");
                            }

                            target_state = target_state_parent;
                        } else {
                            break;
                        }
                    }
                }
            } else {
                while let Some(target_state_parent) = target_state.parent {
                    entry_path.push(target_state_parent).expect("Unexpectedly exceeded path capacity");
                    target_state = target_state_parent;
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
