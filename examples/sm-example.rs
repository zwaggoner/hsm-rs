extern crate rsm;

use rsm::{Action, Actor, Hsm, HsmState, MpmcBoundedQueue, RuntimeState, State, Step};

#[derive(Debug)]
enum UserEvent {
    TestEvent,
}

#[derive(Debug)]
struct TestActor {}

impl Hsm for TestActor {
    type Event = UserEvent;

    fn initial(&mut self) -> State<Self> {
        Top::state()
    }
}

struct Top;

impl HsmState<Top> for TestActor {
    fn initial(&mut self) -> Option<State<Self>> {
        println!("Top State Initial");

        Some(State1::state())
    }

    fn entry(&mut self) {
        println!("Top State Entry");
    }
}

struct State1;

impl HsmState<State1> for TestActor {
    const PARENT: Option<State<Self>> = Some(&Top::STATE);

    fn entry(&mut self) {
        println!("State1 Entry");
    }

    fn handler(&mut self, _event: &UserEvent) -> Action<Self> {
        Action::<Self>::Transition(&State2::STATE)
    }

    fn exit(&mut self) {
        println!("State1 Exit");
    }
}

struct State2;

impl HsmState<State2> for TestActor {
    const PARENT: Option<State<Self>> = Some(&Top::STATE);

    fn entry(&mut self) {
        println!("State2 Entry");
    }

    fn handler(&mut self, _event: &UserEvent) -> Action<Self> {
        Action::<Self>::Transition(State1::state())
    }

    fn exit(&mut self) {
        println!("State2 Exit");
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
