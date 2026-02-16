extern crate rsm;

use rsm::{StateAction, State, StateMachine, state};

#[derive(Debug)]
enum UserEvent {
    TestEvent,
}

#[derive(Debug)]
struct ActorCtx {}

struct Actor {
    sm: StateMachine<State1>,
    context: ActorCtx,
}

state! {
    impl Top {
        type Context = ActorCtx;
        type Event = UserEvent;

        fn initial(_context : &mut ActorCtx) -> Option<State::<Self>> {
            println!("Top State Initial");

            Some(state!(runtime State1))
        }

        fn entry(_context : &mut ActorCtx) {
            println!("Top State Entry");
        }
    }
}

state! {
    impl State1 {
        type Context = ActorCtx;
        type Event = UserEvent;

        const PARENT : Option<State::<Self>> = Some(state!(runtime Top));

        fn entry(_context : &mut ActorCtx) {
            println!("State1 Entry");
        }

        fn handler(_context: &mut ActorCtx, _event : &UserEvent) -> StateAction<Self> {
            StateAction::<Self>::Transition(state!(runtime State2))
        }

        fn exit(_context : &mut ActorCtx) {
            println!("State1 Exit");
        }
    }
}

state! {
    impl State2 {
        type Context = ActorCtx;
        type Event = UserEvent;

        const PARENT : Option<State::<Self>> = Some(state!(runtime Top));

        fn entry(_context : &mut ActorCtx) {
            println!("State2 Entry");
        }

        fn handler(_context: &mut ActorCtx, _event : &UserEvent) -> StateAction<Self> {
            StateAction::<Self>::Transition(state!(runtime State1))
        }

        fn exit(_context : &mut ActorCtx) {
            println!("State2 Exit");
        }
    }
}

fn main() {
    let mut actor = Actor {
        sm: StateMachine::default(),
        context: ActorCtx {},
    };

    {
        actor.sm.run(&mut actor.context);
    }

    for _ in 0..3 {
        actor.sm.dispatch(&mut actor.context, &UserEvent::TestEvent);
    }
}
