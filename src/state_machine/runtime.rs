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
///
/// Once you have defined your state machine using a combination of implementing [`StateMachineDef`] and
/// `StateDef`, you can either use the `actor` framework to coordinate event delivery, or
/// dispatch events manually to your active object (actor) like below:
///
/// ```
/// # use hsm_rs::*;
/// #
/// # enum MyEvent {
/// #    Event1,
/// #    Event2,
/// # }
/// #
/// # struct MyActor;
/// # impl MyActor {
/// #   fn new() -> Self {
/// #       Self {}
/// #   }
/// # }
/// #
/// # impl StateMachineDef for MyActor {
/// #    type Event = MyEvent;
/// #
/// #    fn initial(&mut self) -> State<Self> {
/// #        State1::state()
/// #    }
/// # }
/// #
/// # struct State1;
/// # impl StateDef<State1> for MyActor {
/// #    type Parent = Top;
/// #
/// # }
/// fn main () {
///    let mut my_actor = MyActor::new();
///    let mut sm = StateMachine::new().initial(&mut my_actor);
///
///    sm.dispatch(&mut my_actor, &MyEvent::Event1);
/// }
/// ```
pub struct StateMachine<Sm: StateMachineDef + 'static, S: RunState = Init> {
    curr_state: Option<State<Sm>>,
    _pd: PhantomData<S>,
}

impl<Sm: StateMachineDef, S: RunState> StateMachine<Sm, S> {
    /// Exits up to the `target` state, returns `true` if any states were exited, `false` if no
    /// action occurred.
    fn exit_to(&mut self, context: &mut Sm, target: State<Sm>) -> bool {
        let mut exited: bool = false;

        while let Some(curr_state) = self.curr_state {
            if curr_state.depth() > target.depth() {
                curr_state.exit(context);
                exited = true;
                self.curr_state = curr_state.parent();
            } else {
                break;
            }
        }

        exited
    }

    /// Finds the path to the `target` that is deeper than the current state, as any states between
    /// current state depth and target state depth will need to be entered regardless of the
    /// topology of the state machine.
    fn get_path_to_target(&self, target: State<Sm>) -> StatePath<Sm> {
        let depth: usize = self.curr_state.map_or(0, State::depth);

        let mut entry_path: StatePath<Sm> = StatePath::<Sm>::new();

        if let Some(mut curr_target_path_state) = target.parent() {
            while curr_target_path_state.depth() > depth {
                entry_path
                    .push(curr_target_path_state)
                    .expect("Unexpectedly exceeded path capacity");

                if let Some(curr_target_path_state_parent) = curr_target_path_state.parent() {
                    curr_target_path_state = curr_target_path_state_parent;
                } else {
                    break;
                }
            }
        }

        entry_path
    }

    /// Simultaneously calculates the entry path and exits up to the LCA, given the preconditions
    /// that:
    /// - The current state depth is equal to or one less than the target state
    /// - The `entry_path` is initialized with all states in between the target depth and the current
    ///   state depth.
    ///
    /// In cases of initial transitions to child states, there should only be calls to entry into
    /// child states. Exits can be disabled by setting `allow_exit_to_lca` to false.
    fn get_entry_path_and_exit_to_lca(
        &mut self,
        context: &mut Sm,
        source: Option<State<Sm>>,
        target: State<Sm>,
        entry_path: &mut StatePath<Sm>,
        allow_exit_to_lca: bool,
    ) {
        let source_depth: usize = source.map_or(0, State::depth);
        let mut curr_target_path_state = entry_path.last().map_or(target, |state| *state);

        // Recurse up the state path until the least common ancestor between the source state that
        // initiated the transition and the target state is found. In order to achieve this, the
        // loop checks if a common ancestor has been found, and that the depth of the common
        // ancestor corresponds with having at least traversed up to the depth of the source of the transition.
        while curr_target_path_state.parent() != self.curr_state
            || curr_target_path_state.depth() > (source_depth + 1)
        {
            if let Some(curr_state) = self.curr_state {
                // Fail when exits are disabled but an exit is required for the transition.
                assert!(
                    allow_exit_to_lca,
                    "Invalid Transition: This transition requires an exit, but exits not permitted"
                );

                // Exit from the current state
                curr_state.exit(context);
                self.curr_state = curr_state.parent();

                // In all remaining cases not otherwise handled by the existing transition logic, the current
                // state will always need to be exited. Therefore, we prospectively exit the current
                // state when traversing to find the LCA. If the current and target are at the same
                // depth, the exit will have executed, and the current and target will be one depth
                // different from eachother.
                if curr_target_path_state.depth() == curr_state.depth() {
                    continue;
                }

                // Check that the assumptions documented above hold, if not, this function will not
                // calculate the LCA and entry path appropriately.
                assert!(
                    curr_target_path_state.depth() == curr_state.depth() + 1,
                    "Invalid target and source tree configuration detected"
                );
            }

            if let Some(curr_target_path_state_parent) = curr_target_path_state.parent() {
                entry_path
                    .push(curr_target_path_state_parent)
                    .expect("Unexpectedly exceeded path capacity");

                curr_target_path_state = curr_target_path_state_parent;
            } else {
                break;
            }
        }
    }

    /// Execute a transition from `source` to `target`. `source` can be none when the state machine
    /// has no active state, which should only be the very first initial transition.
    fn transition(&mut self, context: &mut Sm, source: Option<State<Sm>>, target: State<Sm>) {
        let mut child_initial_transition = false;
        let mut transition_target = Some(target);
        let mut transition_source = source;

        while let Some(target_state) = transition_target {
            let exited_from_curr = {
                // If this is an initial transition, we already know from the assertion that the
                // target child is deeper than the parent, the exit_to call will be a no-op.
                // Therefore, do not execute the exit_to call and risk executing any illegal exits.
                if child_initial_transition {
                    false
                } else {
                    // Prospectively exit up to the target_state, since anything below the target
                    // will need to be exited anyways.
                    self.exit_to(context, target_state)
                }
            };

            // Find the entry path in excess of the current state
            let mut entry_path: StatePath<Sm> = self.get_path_to_target(target_state);
            let mut enter_target: bool = true;

            if let Some(curr_state) = self.curr_state
                && transition_target == transition_source
                && curr_state == target_state
            {
                // This is a transition to self, and is treated as an external transition.
                curr_state.exit(context);
                self.curr_state = curr_state.parent();
            } else if let Some(curr_state) = self.curr_state
                && curr_state == target_state
                && exited_from_curr
            {
                // This is a transition to an ancestor, and is treated as an internal transition.
                enter_target = false;
            } else {
                // In all other cases, the current state depth and target state depth are now
                // aligned appropriately to traverse upwards and simultaneously calculate the entry
                // path and exit up to the LCA
                self.get_entry_path_and_exit_to_lca(
                    context,
                    transition_source,
                    target_state,
                    &mut entry_path,
                    !child_initial_transition,
                );
            }

            // Enter all states in the entry path
            for state in entry_path.iter().rev() {
                state.entry(context);
            }

            // Only enter target if appropriate
            if enter_target {
                target_state.entry(context);
            }

            // Check for initial transitions in new target state.
            transition_target = target_state.initial(context);
            transition_source = Some(target_state);
            child_initial_transition = true;

            // Mark the transition as complete
            self.curr_state = Some(target_state);

            // All initial transitions should be to a valid child, if the depth is not greater than
            // the current state (still the target_state variable in this case), the initial
            // transition is invalid.
            if let Some(tt) = transition_target {
                assert!(
                    tt.depth() > target_state.depth(),
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
    /// Instantiates a new state machine
    #[must_use]
    pub fn new() -> Self {
        Self {
            curr_state: None,
            _pd: PhantomData::<Init>,
        }
    }

    /// Performs the topmost initial transition for the `StateMachine` as specified by the
    /// [`StateMachineDef`] initial transition. Note that `StateMachine` follows the typestate
    /// pattern, so the call to `initial` consumes and returns a new `StateMachine` object.
    ///
    /// # Panics
    /// Should the initial transition be illegal for any of the reasons documented under [`StateMachine::dispatch`],
    /// the call to `initial` will panic.  
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
    /// Dispatches an event to the `StateMachine`. Events are eligible to handle from the deepest
    /// state in the hierarchy (child states), up to the parent states until one handles the event.
    /// Should an event be unhandled, it will be silently discarded. If other behavior is desired, a
    /// topmost state handler may be added to respond to unhandled states appropriately.
    ///
    /// # Panics
    /// Compile time checks for the state machine framework guarantee that there are no cycles in
    /// the parent specification, and the depth of all state machines are bounded. That said if any
    /// depth violations occurred, they would result in a panic.
    ///
    /// The one violation that cannot be checked at compile time is an incorrect initial transition
    /// specification. Initial transitions must always be to a child of the current state. If they
    /// are not, the transition logic will panic, and hence the call to dispatch performing the
    /// illegal transition will panic.
    pub fn dispatch(&mut self, context: &mut Sm, event: &Sm::Event) {
        let mut state_opt = self.curr_state;

        while let Some(state) = state_opt {
            match state.handler(context, event) {
                Action::<Sm>::Handled => break,
                Action::<Sm>::Transition(new_state) => {
                    self.transition(context, Some(state), new_state);
                    break;
                }
                Action::Unhandled => (),
            }

            state_opt = state.parent();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StatePath;
    use crate::{State, StateDef, StateMachine, StateMachineDef, StateRef, Super, Top};

    struct TestActor;

    enum TestEvent {}

    struct S1;
    struct S2;
    struct S21;
    struct S211;

    impl StateMachineDef for TestActor {
        type Event = TestEvent;

        fn initial(&mut self) -> State<Self> {
            S1::state()
        }
    }

    impl StateDef<S1> for TestActor {
        type Parent = Top;
    }

    impl StateDef<S2> for TestActor {
        type Parent = Top;
    }

    impl StateDef<S21> for TestActor {
        type Parent = Super<S2>;
    }

    impl StateDef<S211> for TestActor {
        type Parent = Super<S21>;
    }

    #[test]
    #[should_panic]
    fn test_invalid_configuration() {
        let mut actor = TestActor {};
        let mut sm = StateMachine::<TestActor>::default().initial(&mut actor);

        let sm_curr = sm.curr_state;
        let mut entry_path: StatePath<TestActor> = StatePath::<TestActor>::new();

        sm.get_entry_path_and_exit_to_lca(
            &mut actor,
            sm_curr,
            S211::state(),
            &mut entry_path,
            false,
        );
    }
}
