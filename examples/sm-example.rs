extern crate rsm;

use rsm::{
    Action, State, StateDef, StateMachineDef, StateRef, Super, Top,
    actor::{Actor, ActorRuntime, queue::MpmcBoundedQueue},
};

#[derive(Debug)]
enum UserEvent {
    TestEvent,
}

#[derive(Debug)]
struct TestActor {}

impl StateMachineDef for TestActor {
    type Event = UserEvent;

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

    fn handler(&mut self, _event: &UserEvent) -> Action<Self> {
        Action::Transition(State12::state())
    }

    fn exit(&mut self) {
        println!("State11 Exit");
    }
}

struct State12;

impl StateDef<State12> for TestActor {
    type Parent = Super<State1>;

    fn initial(&mut self) -> Option<State<Self>> {
        println!("State12 Initial");

        None
    }

    fn entry(&mut self) {
        println!("State12 Entry");
    }

    fn handler(&mut self, _event: &UserEvent) -> Action<Self> {
        Action::Transition(State11::state())
    }

    fn exit(&mut self) {
        println!("State12 Exit");
    }
}

fn main() {
    let actor = Actor::<TestActor, MpmcBoundedQueue<UserEvent, 32>>::new(MpmcBoundedQueue::<
        UserEvent,
        32,
    >::default());

    actor.enqueue(UserEvent::TestEvent).unwrap();
    actor.enqueue(UserEvent::TestEvent).unwrap();
    actor.enqueue(UserEvent::TestEvent).unwrap();

    let mut actor_rt = actor.bind(TestActor {});

    while actor_rt.step().did_work() {}
}
