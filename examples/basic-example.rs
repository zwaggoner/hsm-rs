use hsm_rs::{Action, State, StateDef, StateMachine, StateMachineDef, StateRef, Super, Top};

enum TestEvent {
    Event1,
    Event2,
}

struct TestActor;

impl StateMachineDef for TestActor {
    type Event = TestEvent;

    fn initial(&mut self) -> State<Self> {
        println!("TestActor Initial");
        State1::state()
    }
}

struct State1;

impl StateDef<State1> for TestActor {
    type Parent = Top;

    fn initial(&mut self) -> Option<State<Self>> {
        println!("State1 Initial");

        Some(State11::state())
    }

    fn entry(&mut self) {
        println!("State1 Entry");
    }

    fn handler(&mut self, event: &TestEvent) -> Action<Self> {
        println!("State1 Handler");

        match event {
            TestEvent::Event1 => Action::Handled,
            _ => Action::Unhandled,
        }
    }

    fn exit(&mut self) {
        println!("State1 Exit");
    }
}

struct State11;

impl StateDef<State11> for TestActor {
    type Parent = Super<State1>;

    fn initial(&mut self) -> Option<State<Self>> {
        println!("State11 Initial");

        None
    }

    fn entry(&mut self) {
        println!("State11 Entry");
    }

    fn handler(&mut self, event: &TestEvent) -> Action<Self> {
        println!("State11 Handler");

        match event {
            TestEvent::Event2 => Action::Transition(State2::state()),
            _ => Action::Unhandled,
        }
    }

    fn exit(&mut self) {
        println!("State11 Exit");
    }
}

struct State2;

impl StateDef<State2> for TestActor {
    type Parent = Top;

    fn initial(&mut self) -> Option<State<Self>> {
        println!("State2 Initial");

        None
    }

    fn entry(&mut self) {
        println!("State2 Entry");
    }

    fn handler(&mut self, event: &TestEvent) -> Action<Self> {
        println!("State2 Handler");

        match event {
            TestEvent::Event1 => Action::Transition(State11::state()),
            _ => Action::Unhandled,
        }
    }

    fn exit(&mut self) {
        println!("State2 Exit");
    }
}

fn main() {
    let mut test_actor = TestActor {};
    let mut sm = StateMachine::new().initial(&mut test_actor);

    // Handled Event
    sm.dispatch(&mut test_actor, &TestEvent::Event1);

    // Transition to State2
    sm.dispatch(&mut test_actor, &TestEvent::Event2);

    // Unhandled Event
    sm.dispatch(&mut test_actor, &TestEvent::Event2);

    // Transition back to State 11
    sm.dispatch(&mut test_actor, &TestEvent::Event1);
}
