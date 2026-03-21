extern crate rsm;

use rsm::{Action, Actor, AsState, Hsm, HsmState, MpmcBoundedQueue, RuntimeState, State, Step, Top};

#[derive(Debug)]
enum UserEvent {
    TestEvent,
}

#[derive(Debug)]
struct TestActor {}

impl Hsm for TestActor {
    type Event = UserEvent;

    fn initial(&mut self) -> State<Self> {
        State1::state()
    }
}

struct State1;

impl HsmState<State1> for TestActor {
    type Parent = Top;

    fn initial(&mut self) -> Option<State<Self>> {
        println!("State1 Initial");

        Some(State11::state())
    }

    fn entry(&mut self) {
        println!("State1 Entry");
    }
}

struct State11;

impl HsmState<State11> for TestActor {
    type Parent = AsState<State1>;

    fn entry(&mut self) {
        println!("State11 Entry");
    }

    fn handler(&mut self, _event: &UserEvent) -> Action<Self> {
        Action::<Self>::Transition(State12::state())
    }

    fn exit(&mut self) {
        println!("State11 Exit");
    }
}

struct State12;

impl HsmState<State12> for TestActor {
    type Parent = AsState<State1>;

    fn entry(&mut self) {
        println!("State12 Entry");
    }

    fn handler(&mut self, _event: &UserEvent) -> Action<Self> {
        Action::<Self>::Transition(State11::state())
    }

    fn exit(&mut self) {
        println!("State12 Exit");
    }
}

fn main() {
    let context = TestActor {};

    let mut actor = Actor::<TestActor, MpmcBoundedQueue<UserEvent, 32>>::new(
        context,
        MpmcBoundedQueue::<UserEvent, 32>::default(),
    );

    let producer = actor.event_producer();

    producer.enqueue(UserEvent::TestEvent).unwrap();
    producer.enqueue(UserEvent::TestEvent).unwrap();
    producer.enqueue(UserEvent::TestEvent).unwrap();

    while actor.step() {}
}
