extern crate rsm;

use rsm::{Action, Hsm, HsmState, MpmcBoundedQueue, RuntimeState, State, StateMachine};

#[derive(Debug)]
enum UserEvent {
    TestEvent,
}

#[derive(Debug)]
struct ActorCtx {}

struct ActorSM;

impl Hsm for ActorSM {
    type Event = UserEvent;
    type Context = ActorCtx;
}

struct Actor {
    sm: StateMachine<ActorSM, MpmcBoundedQueue<UserEvent, 32>>,
    context: ActorCtx,
}

struct Top;

impl HsmState<ActorSM> for Top {
    fn initial(_context: &mut ActorCtx) -> Option<State<ActorSM>> {
        println!("Top State Initial");

        Some(&State1::STATE)
    }

    fn entry(_context: &mut ActorCtx) {
        println!("Top State Entry");
    }
}

struct State1;

impl HsmState<ActorSM> for State1 {
    const PARENT: Option<State<ActorSM>> = Some(&Top::STATE);

    fn entry(_context: &mut ActorCtx) {
        println!("State1 Entry");
    }

    fn handler(_context: &mut ActorCtx, _event: &UserEvent) -> Action<ActorSM> {
        Action::<ActorSM>::Transition(&State2::STATE)
    }

    fn exit(_context: &mut ActorCtx) {
        println!("State1 Exit");
    }
}

struct State2;

impl HsmState<ActorSM> for State2 {
    const PARENT: Option<State<ActorSM>> = Some(&Top::STATE);

    fn entry(_context: &mut ActorCtx) {
        println!("State2 Entry");
    }

    fn handler(_context: &mut ActorCtx, _event: &UserEvent) -> Action<ActorSM> {
        Action::<ActorSM>::Transition(&State1::STATE)
    }

    fn exit(_context: &mut ActorCtx) {
        println!("State2 Exit");
    }
}

fn main() {
    let mut actor = Actor {
        sm: StateMachine::new(),
        context: ActorCtx {},
    };

    let producer = actor.sm.event_producer();

    producer.enqueue(UserEvent::TestEvent).unwrap();
    producer.enqueue(UserEvent::TestEvent).unwrap();
    producer.enqueue(UserEvent::TestEvent).unwrap();

    let mut sm = actor.sm.initial(&mut actor.context, Top::state());

    sm.step_all(&mut actor.context);
}
