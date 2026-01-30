extern crate rsm;

use rsm::{Event, Action, StateMachine};

#[derive(Clone, Copy)]
enum UserEvents {
    TestEvent
}

struct ActorCtx {}

struct Actor {
    sm : StateMachine<ActorCtx, UserEvents>,
    context : ActorCtx
}

impl Actor {
    fn top_state(context: &mut ActorCtx, event: Event<UserEvents>) -> Action<ActorCtx, UserEvents> {
        match event {
            Event::Entry => {
                println!("Top State Entry");
                Action::<ActorCtx, UserEvents>::Handled
            },
            Event::Exit => {
                println!("Top State Exit");
                Action::<ActorCtx, UserEvents>::Handled
            },
            _ =>  {
                Action::<ActorCtx, UserEvents>::Unhandled
            }
        }
    }

    fn state1(context : &mut ActorCtx, event: Event<UserEvents>) -> Action::<ActorCtx, UserEvents> {
        match event {
            Event::GetParent => Action::<ActorCtx, UserEvents>::Parent(Self::top_state),
            Event::Entry => {
                println!("State1 Entry");
                Action::<ActorCtx, UserEvents>::Handled
            },
            Event::Exit => {
                println!("State1 Exit");
                Action::<ActorCtx, UserEvents>::Handled
            }
            Event::Other(user) => {
                match user {
                   UserEvents::TestEvent => Action::<ActorCtx, UserEvents>::Transition(Self::state2),
                   _ => Action::<ActorCtx, UserEvents>::Unhandled,
                }
            },
            _ => Action::<ActorCtx, UserEvents>::Unhandled
        }
    }

    fn state2(context : &mut ActorCtx, event: Event<UserEvents>) -> Action::<ActorCtx, UserEvents> {
        match event {
            Event::GetParent => Action::<ActorCtx, UserEvents>::Parent(Self::top_state),
            Event::Entry => {
                println!("State2 Entry");
                Action::<ActorCtx, UserEvents>::Handled
            },
            Event::Exit => {
                println!("State2 Exit");
                Action::<ActorCtx, UserEvents>::Handled
            }
            Event::Other(user) => {
                match user {
                   UserEvents::TestEvent => Action::<ActorCtx, UserEvents>::Transition(Self::state2),
                   _ => Action::<ActorCtx, UserEvents>::Unhandled,
                }
            },
            _ => Action::<ActorCtx, UserEvents>::Unhandled
        }
    }
}

fn main() {
    let mut actor = Actor { sm : StateMachine::default(), context : ActorCtx{} };

    {
        actor.sm.initial(&mut actor.context, Actor::state1);
    }

    actor.sm.dispatch(&mut actor.context, Event::<UserEvents>::Other(UserEvents::TestEvent));
}
