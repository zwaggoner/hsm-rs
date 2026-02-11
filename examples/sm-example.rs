extern crate rsm;

use rsm::{Action, Event, StateMachine};

#[derive(Clone, Copy)]
enum UserEvents {
    TestEvent,
}

struct ActorCtx {}

struct Actor {
    sm: StateMachine<ActorCtx, UserEvents>,
    context: ActorCtx,
}

impl Actor {
    fn top_state(
        _context: &mut ActorCtx,
        event: Event<UserEvents>,
    ) -> Action<ActorCtx, UserEvents> {
        match event {
            Event::Entry => {
                println!("Top State Entry");
                Action::Handled
            }
            Event::Exit => {
                println!("Top State Exit");
                Action::Handled
            }
            _ => Action::Unhandled,
        }
    }

    fn state1(_context: &mut ActorCtx, event: Event<UserEvents>) -> Action<ActorCtx, UserEvents> {
        match event {
            Event::Parent(action) => action.parent(Self::top_state),
            Event::Entry => {
                println!("State1 Entry");
                Action::Handled
            }
            Event::Exit => {
                println!("State1 Exit");
                Action::Handled
            }
            Event::Other{ event : user, action }  => match user {
                UserEvents::TestEvent => action.transition(Self::state2),
            },
            _ => Action::Unhandled,
        }
    }

    fn state2(_context: &mut ActorCtx, event: Event<UserEvents>) -> Action<ActorCtx, UserEvents> {
        match event {
            Event::Parent(action) => action.parent(Self::top_state),
            Event::Entry => {
                println!("State2 Entry");
                Action::Handled
            }
            Event::Exit => {
                println!("State2 Exit");
                Action::Handled
            }
            Event::Other{ event : user, action }  => match user {
                UserEvents::TestEvent => action.transition(Self::state1),
            },
            _ => Action::Unhandled,
        }
    }
}

fn main() {
    let mut actor = Actor {
        sm: StateMachine::default(),
        context: ActorCtx {},
    };

    {
        actor.sm.initial(&mut actor.context, Actor::state1);
    }

    for _ in 0..3 {
        actor.sm.dispatch(
            &mut actor.context,
            UserEvents::TestEvent
        );
    }
}
