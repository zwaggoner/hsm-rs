extern crate rsm;

use rsm::{Action, State, StateMachine, state};

enum UserEvent {
    TestEvent,
}

struct ActorCtx {}

struct Actor {
    sm: StateMachine<ActorCtx, UserEvent>,
    context: ActorCtx,
}

state! {
    impl Top {
        type Context = ActorCtx;
        type Event = UserEvent;

        fn initial(_context : &mut ActorCtx) -> Option<State::<ActorCtx, UserEvent>> {
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

        const PARENT : Option<State::<ActorCtx, UserEvent>> = Some(state!(runtime Top));

        fn entry(_context : &mut ActorCtx) {
            println!("State1 Entry");
        }

        fn handler(_context: &mut ActorCtx, _event : &UserEvent) -> Action<ActorCtx, UserEvent> {
            Action::Transition(state!(runtime State2))
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

        const PARENT : Option<State::<ActorCtx, UserEvent>> = Some(state!(runtime Top));

        fn entry(_context : &mut ActorCtx) {
            println!("State2 Entry");
        }

        fn handler(_context: &mut ActorCtx, _event : &UserEvent) -> Action<ActorCtx, UserEvent> {
            Action::Transition(state!(runtime State1))
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
        actor.sm.initial(&mut actor.context, state!(runtime Top));
    }

    for _ in 0..3 {
        actor.sm.dispatch(&mut actor.context, &UserEvent::TestEvent);
    }
}
