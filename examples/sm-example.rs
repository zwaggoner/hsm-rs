extern crate rsm;

use rsm::StateMachine;

#[derive(Copy, Clone, Debug)]
enum UserEvent {
    TestEvent,
}

#[derive(Debug)]
struct ActorCtx {}

struct Actor {
    sm: StateMachine<ActorCtx, UserEvent>,
    context: ActorCtx,
}

rsm::state! {
    impl Top {
        type Context = ActorCtx;
        type Event = UserEvent;

        fn initial(_context : &mut ActorCtx) -> Option<rsm::State::<ActorCtx, UserEvent>> {
            println!("Top State Initial");

            Some(rsm::state!(runtime State1))
        }

        fn entry(_context : &mut ActorCtx) {
            println!("Top State Entry");
        }
    }
}

rsm::state! {
    impl State1 {
        type Context = ActorCtx;
        type Event = UserEvent;

        const PARENT : Option<rsm::State::<ActorCtx, UserEvent>> = Some(rsm::state!(runtime Top));

        fn entry(_context : &mut ActorCtx) {
            println!("State1 Entry");
        }

        fn handler(_context: &mut ActorCtx, _event : UserEvent) -> rsm::Action<ActorCtx, UserEvent> {
            rsm::Action::Transition(rsm::state!(runtime State2))
        }

        fn exit(_context : &mut ActorCtx) {
            println!("State1 Exit");
        }
    }
}

rsm::state! {
    impl State2 {
        type Context = ActorCtx;
        type Event = UserEvent;

        const PARENT : Option<rsm::State::<ActorCtx, UserEvent>> = Some(rsm::state!(runtime Top));

        fn entry(_context : &mut ActorCtx) {
            println!("State2 Entry");
        }

        fn handler(_context: &mut ActorCtx, _event : UserEvent) -> rsm::Action<ActorCtx, UserEvent> {
            rsm::Action::Transition(rsm::state!(runtime State1))
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
        actor
            .sm
            .initial(&mut actor.context, rsm::state!(runtime Top));
    }

    for _ in 0..3 {
        actor.sm.dispatch(&mut actor.context, UserEvent::TestEvent);
    }
}
