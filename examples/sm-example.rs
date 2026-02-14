extern crate rsm;

use rsm::{StateMachine};

#[derive(Copy, Clone)]
enum UserEvent {
    TestEvent,
}

struct ActorCtx {}

struct Actor {
    sm: StateMachine<ActorCtx, UserEvent>,
    context: ActorCtx,
}

rsm::state!{
    impl Top<ActorCtx, UserEvent> {
        fn initial(_context : &mut ActorCtx) -> Option<rsm::State::<ActorCtx, UserEvent>> {
            println!("Top State Initial");

            Some(rsm::state!(runtime State1::<ActorCtx, UserEvent>))
        }

        fn entry(_context : &mut ActorCtx) {
            println!("Top State Entry");
        }
    }
}

rsm::state!{
    impl State1<ActorCtx, UserEvent> {
        fn entry(_context : &mut ActorCtx) {
            println!("State1 Entry");
        }

        fn handler(_context: &mut ActorCtx, _event : UserEvent) -> rsm::Action<ActorCtx, UserEvent> {
            rsm::Action::Transition(rsm::state!(runtime State2<ActorCtx, UserEvent>))
        }

        fn exit(_context : &mut ActorCtx) {
            println!("State1 Exit");
        }
    }
}

rsm::state!{
    impl State2<ActorCtx, UserEvent> {
        fn entry(_context : &mut ActorCtx) {
            println!("State2 Entry");
        }
    }
}

fn main() {
    let mut actor = Actor {
        sm: StateMachine::default(),
        context: ActorCtx {},
    };

    {
        actor.sm.initial(&mut actor.context, rsm::state!(runtime Top<ActorCtx, UserEvent>));
    }
    
    actor.sm.dispatch(&mut actor.context, UserEvent::TestEvent);
}
