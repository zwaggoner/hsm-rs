extern crate hsm_rs;
extern crate std;

use hsm_rs::{Action, State, StateDef, StateMachine, StateMachineDef, StateRef, Super, Top};

use std::vec::Vec;

enum TestEvent {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
}

#[derive(Debug, PartialEq)]
enum Trace {
    TopInitial,
    S0Entry,
    S0Initial,
    S0Handler,
    S0Exit,
    S1Entry,
    S1Initial,
    S1Handler,
    S1Exit,
    S11Entry,
    S11Initial,
    S11Handler,
    S11Exit,
    S2Entry,
    S2Initial,
    S2Handler,
    S2Exit,
    S21Entry,
    S21Initial,
    S21Handler,
    S21Exit,
    S211Entry,
    S211Initial,
    S211Handler,
    S211Exit,
}

enum InitialTransitionTestType {
    None,
    Deep,
    InvalidSelf,
    InvalidSibling,
    InvalidAncestor,
}

struct TestActor {
    trace: Vec<Trace>,
    foo: bool,
    initial_transition_test_type: InitialTransitionTestType,
}

impl TestActor {
    fn new(initial_transition_test_type: InitialTransitionTestType) -> Self {
        Self {
            trace: Vec::new(),
            foo: false,
            initial_transition_test_type,
        }
    }

    fn record(&mut self, trace: Trace) {
        self.trace.push(trace);
    }
}

impl Default for TestActor {
    fn default() -> Self {
        Self::new(InitialTransitionTestType::None)
    }
}

impl StateMachineDef for TestActor {
    type Event = TestEvent;

    fn initial(&mut self) -> State<Self> {
        self.record(Trace::TopInitial);

        match self.initial_transition_test_type {
            InitialTransitionTestType::Deep => S211::state(),
            _ => S0::state(),
        }
    }
}

struct S0;
struct S1;
struct S11;
struct S2;
struct S21;
struct S211;

impl StateDef<S0> for TestActor {
    type Parent = Top;

    fn initial(&mut self) -> Option<State<Self>> {
        self.record(Trace::S0Initial);

        Some(S1::state())
    }

    fn entry(&mut self) {
        self.record(Trace::S0Entry);
    }

    fn exit(&mut self) {
        self.record(Trace::S0Exit);
    }

    fn handler(&mut self, event: &TestEvent) -> Action<Self> {
        self.record(Trace::S0Handler);

        match event {
            TestEvent::E => Action::Transition(S211::state()),
            _ => Action::Unhandled,
        }
    }
}

impl StateDef<S1> for TestActor {
    type Parent = Super<S0>;

    fn initial(&mut self) -> Option<State<Self>> {
        self.record(Trace::S1Initial);

        match self.initial_transition_test_type {
            InitialTransitionTestType::InvalidSelf => Some(S1::state()),
            InitialTransitionTestType::InvalidSibling => Some(S2::state()),
            InitialTransitionTestType::InvalidAncestor => Some(S0::state()),
            _ => Some(S11::state()),
        }
    }

    fn entry(&mut self) {
        self.record(Trace::S1Entry);
    }

    fn exit(&mut self) {
        self.record(Trace::S1Exit);
    }

    fn handler(&mut self, event: &TestEvent) -> Action<Self> {
        self.record(Trace::S1Handler);

        match event {
            TestEvent::A => Action::Transition(S1::state()),
            TestEvent::B => Action::Transition(S11::state()),
            TestEvent::C => Action::Transition(S2::state()),
            TestEvent::D => Action::Transition(S0::state()),
            TestEvent::F => Action::Transition(S211::state()),
            _ => Action::Unhandled,
        }
    }
}

impl StateDef<S11> for TestActor {
    type Parent = Super<S1>;

    fn initial(&mut self) -> Option<State<Self>> {
        self.record(Trace::S11Initial);

        None
    }

    fn entry(&mut self) {
        self.record(Trace::S11Entry);
    }

    fn exit(&mut self) {
        self.record(Trace::S11Exit);
    }

    fn handler(&mut self, event: &TestEvent) -> Action<Self> {
        self.record(Trace::S11Handler);

        match event {
            TestEvent::H if self.foo => {
                self.foo = false;
                Action::Handled
            }
            TestEvent::G => Action::Transition(S211::state()),
            _ => Action::Unhandled,
        }
    }
}

impl StateDef<S2> for TestActor {
    type Parent = Super<S0>;

    fn initial(&mut self) -> Option<State<Self>> {
        self.record(Trace::S2Initial);

        Some(S21::state())
    }

    fn entry(&mut self) {
        self.record(Trace::S2Entry);
    }

    fn exit(&mut self) {
        self.record(Trace::S2Exit);
    }

    fn handler(&mut self, event: &TestEvent) -> Action<Self> {
        self.record(Trace::S2Handler);

        match event {
            TestEvent::C => Action::Transition(S1::state()),
            TestEvent::F => Action::Transition(S11::state()),
            _ => Action::Unhandled,
        }
    }
}

impl StateDef<S21> for TestActor {
    type Parent = Super<S2>;

    fn initial(&mut self) -> Option<State<Self>> {
        self.record(Trace::S21Initial);

        Some(S211::state())
    }

    fn entry(&mut self) {
        self.record(Trace::S21Entry);
    }

    fn exit(&mut self) {
        self.record(Trace::S21Exit);
    }

    fn handler(&mut self, event: &TestEvent) -> Action<Self> {
        self.record(Trace::S21Handler);

        match event {
            TestEvent::B => Action::Transition(S211::state()),
            TestEvent::H if !self.foo => {
                self.foo = true;
                Action::Transition(S21::state())
            }
            _ => Action::Unhandled,
        }
    }
}

impl StateDef<S211> for TestActor {
    type Parent = Super<S21>;

    fn initial(&mut self) -> Option<State<Self>> {
        self.record(Trace::S211Initial);

        None
    }

    fn entry(&mut self) {
        self.record(Trace::S211Entry);
    }

    fn exit(&mut self) {
        self.record(Trace::S211Exit);
    }

    fn handler(&mut self, event: &TestEvent) -> Action<Self> {
        self.record(Trace::S211Handler);

        match event {
            TestEvent::D => Action::Transition(S21::state()),
            TestEvent::G => Action::Transition(S0::state()),
            _ => Action::Unhandled,
        }
    }
}

#[test]
fn test_initial_transitions() {
    let mut test_actor = TestActor::default();
    let mut _sm = StateMachine::default().initial(&mut test_actor);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::TopInitial,
            Trace::S0Entry,
            Trace::S0Initial,
            Trace::S1Entry,
            Trace::S1Initial,
            Trace::S11Entry,
            Trace::S11Initial,
        ]
    );
}

#[test]
fn test_self_transition_unguarded() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::A);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::S11Handler,
            Trace::S1Handler,
            Trace::S11Exit,
            Trace::S1Exit,
            Trace::S1Entry,
            Trace::S1Initial,
            Trace::S11Entry,
            Trace::S11Initial,
        ]
    );
}

#[test]
fn test_transition_to_child() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::B);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::S11Handler,
            Trace::S1Handler,
            Trace::S11Exit,
            Trace::S11Entry,
            Trace::S11Initial,
        ]
    );
}

#[test]
fn test_transition_from_nested_sibling_to_deeply_nested_sibling() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::C);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::S11Handler,
            Trace::S1Handler,
            Trace::S11Exit,
            Trace::S1Exit,
            Trace::S2Entry,
            Trace::S2Initial,
            Trace::S21Entry,
            Trace::S21Initial,
            Trace::S211Entry,
            Trace::S211Initial,
        ]
    );
}

#[test]
fn test_transition_to_parent() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::D);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::S11Handler,
            Trace::S1Handler,
            Trace::S11Exit,
            Trace::S1Exit,
            Trace::S0Initial,
            Trace::S1Entry,
            Trace::S1Initial,
            Trace::S11Entry,
            Trace::S11Initial,
        ]
    );
}

#[test]
fn test_transition_from_top_to_great_grandchild_from_other() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::E);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::S11Handler,
            Trace::S1Handler,
            Trace::S0Handler,
            Trace::S11Exit,
            Trace::S1Exit,
            Trace::S2Entry,
            Trace::S21Entry,
            Trace::S211Entry,
            Trace::S211Initial,
        ]
    );
}

#[test]
fn test_transition_to_great_niece_nephew() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::F);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::S11Handler,
            Trace::S1Handler,
            Trace::S11Exit,
            Trace::S1Exit,
            Trace::S2Entry,
            Trace::S21Entry,
            Trace::S211Entry,
            Trace::S211Initial,
        ]
    );
}

#[test]
fn test_transition_to_first_cousin_once_removed() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::G);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::S11Handler,
            Trace::S11Exit,
            Trace::S1Exit,
            Trace::S2Entry,
            Trace::S21Entry,
            Trace::S211Entry,
            Trace::S211Initial,
        ]
    );
}

#[test]
fn test_unhandled_event_guard() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::H);

    assert_eq!(
        test_actor.trace,
        vec![Trace::S11Handler, Trace::S1Handler, Trace::S0Handler,]
    );
}

#[test]
fn test_unhandled_event() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);
    sm.dispatch(&mut test_actor, &TestEvent::C);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::A);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::S211Handler,
            Trace::S21Handler,
            Trace::S2Handler,
            Trace::S0Handler,
        ]
    );
}

#[test]
fn test_transition_to_deeply_nested_child() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);
    sm.dispatch(&mut test_actor, &TestEvent::C);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::B);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::S211Handler,
            Trace::S21Handler,
            Trace::S211Exit,
            Trace::S211Entry,
            Trace::S211Initial,
        ]
    );
}

#[test]
fn test_transition_from_deeply_nested_sibling_to_nested_sibling() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);
    sm.dispatch(&mut test_actor, &TestEvent::C);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::C);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::S211Handler,
            Trace::S21Handler,
            Trace::S2Handler,
            Trace::S211Exit,
            Trace::S21Exit,
            Trace::S2Exit,
            Trace::S1Entry,
            Trace::S1Initial,
            Trace::S11Entry,
            Trace::S11Initial,
        ]
    );
}

#[test]
fn test_transition_from_deeply_nested_child_to_parent() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);
    sm.dispatch(&mut test_actor, &TestEvent::C);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::D);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::S211Handler,
            Trace::S211Exit,
            Trace::S21Initial,
            Trace::S211Entry,
            Trace::S211Initial,
        ]
    );
}

#[test]
fn test_transition_from_top_to_great_grandchild_from_self() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);
    sm.dispatch(&mut test_actor, &TestEvent::C);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::E);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::S211Handler,
            Trace::S21Handler,
            Trace::S2Handler,
            Trace::S0Handler,
            Trace::S211Exit,
            Trace::S21Exit,
            Trace::S2Exit,
            Trace::S2Entry,
            Trace::S21Entry,
            Trace::S211Entry,
            Trace::S211Initial,
        ]
    );
}

#[test]
fn test_transition_to_niece_nephew() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);
    sm.dispatch(&mut test_actor, &TestEvent::C);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::F);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::S211Handler,
            Trace::S21Handler,
            Trace::S2Handler,
            Trace::S211Exit,
            Trace::S21Exit,
            Trace::S2Exit,
            Trace::S1Entry,
            Trace::S11Entry,
            Trace::S11Initial,
        ]
    );
}

#[test]
fn test_transition_to_great_grandparent() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);
    sm.dispatch(&mut test_actor, &TestEvent::C);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::G);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::S211Handler,
            Trace::S211Exit,
            Trace::S21Exit,
            Trace::S2Exit,
            Trace::S0Initial,
            Trace::S1Entry,
            Trace::S1Initial,
            Trace::S11Entry,
            Trace::S11Initial,
        ]
    );
}

#[test]
fn test_self_transition_guard() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);
    sm.dispatch(&mut test_actor, &TestEvent::C);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::H);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::S211Handler,
            Trace::S21Handler,
            Trace::S211Exit,
            Trace::S21Exit,
            Trace::S21Entry,
            Trace::S21Initial,
            Trace::S211Entry,
            Trace::S211Initial,
        ]
    );
}

#[test]
fn test_handled_event_guard() {
    let mut test_actor = TestActor::default();
    let mut sm = StateMachine::default().initial(&mut test_actor);
    sm.dispatch(&mut test_actor, &TestEvent::C);
    sm.dispatch(&mut test_actor, &TestEvent::H);
    sm.dispatch(&mut test_actor, &TestEvent::C);

    test_actor.trace.clear();

    sm.dispatch(&mut test_actor, &TestEvent::H);

    assert_eq!(test_actor.trace, vec![Trace::S11Handler,]);
}

#[test]
fn test_deep_initial() {
    let mut test_actor = TestActor::new(InitialTransitionTestType::Deep);
    let _sm = StateMachine::default().initial(&mut test_actor);

    assert_eq!(
        test_actor.trace,
        vec![
            Trace::TopInitial,
            Trace::S0Entry,
            Trace::S2Entry,
            Trace::S21Entry,
            Trace::S211Entry,
            Trace::S211Initial,
        ]
    );
}

#[test]
#[should_panic(expected = "Initial transitions must be to a valid child state")]
fn test_invalid_initial_transition_to_self() {
    let mut test_actor = TestActor::new(InitialTransitionTestType::InvalidSelf);
    let _sm = StateMachine::default().initial(&mut test_actor);
}

#[test]
#[should_panic(expected = "Initial transitions must be to a valid child state")]
fn test_invalid_initial_transition_to_sibling() {
    let mut test_actor = TestActor::new(InitialTransitionTestType::InvalidSibling);
    let _sm = StateMachine::default().initial(&mut test_actor);
}

#[test]
#[should_panic(expected = "Initial transitions must be to a valid child state")]
fn test_invalid_initial_transition_to_ancestor() {
    let mut test_actor = TestActor::new(InitialTransitionTestType::InvalidAncestor);
    let _sm = StateMachine::default().initial(&mut test_actor);
}

struct DefaultActor;
struct DefaultState;

enum DefaultEvent {
    Event1,
}

impl StateMachineDef for DefaultActor {
    type Event = DefaultEvent;

    fn initial(&mut self) -> State<Self> {
        DefaultState::state()
    }
}

impl StateDef<DefaultState> for DefaultActor {
    type Parent = Top;
}

#[test]
fn test_default_handler() {
    let mut default_actor = DefaultActor {};

    assert!(matches!(
        <DefaultActor as StateDef<DefaultState>>::handler(
            &mut default_actor,
            &DefaultEvent::Event1
        ),
        Action::<DefaultActor>::Unhandled
    ));
}

#[test]
fn test_default_entry() {
    let mut default_actor = DefaultActor {};
    <DefaultActor as StateDef<DefaultState>>::entry(&mut default_actor);
}

#[test]
fn test_default_exit() {
    let mut default_actor = DefaultActor {};
    <DefaultActor as StateDef<DefaultState>>::exit(&mut default_actor);
}

#[test]
fn test_state_clone() {
    assert!(DefaultState::state() == <State<DefaultActor> as Clone>::clone(&DefaultState::state()));
}
