extern crate rsm;

use rsm::{Event, EventAction, StateMachine};

enum UserEvents {
    TestEvent
}

#[derive(Clone)]
#[derive(Copy)]
enum StateId {
    TopState,
    State1,
    State2,
    Max
}

impl rsm::StateId for StateId {
    fn index(self) -> usize {
        self as usize
    }
}

impl Default for StateId {
    fn default() -> Self {
        StateId::Max
    }
}

struct ActorCtx {
}

struct Actor {
    sm : StateMachine<ActorCtx, UserEvents, StateId, { StateId::Max as usize}>,
    ctx : ActorCtx
}

impl Actor {
    fn run(&mut self) {
        self.sm.run(&mut self.ctx)
    }

    fn top_state(actor : &mut ActorCtx, event: Event<UserEvents>) -> EventAction::<StateId> {
        match event {
            Event::Entry => {
                println!("Top State Entry");
                EventAction::<StateId>::Transition(StateId::State1)
            },
            Event::Exit => {
                println!("Top State Exit");
                EventAction::<StateId>::Handled
            }
            _ => {
                //println!("Unhandled Event {}", event);
                EventAction::<StateId>::Unhandled
            }
        }
    }

    fn state1(actor : &mut ActorCtx, event: Event<UserEvents>) -> EventAction::<StateId> {
        match event {
            Event::Entry => {
                println!("State1 Entry");
                EventAction::<StateId>::Handled
            },
            Event::Exit => {
                println!("State1 Exit");
                EventAction::<StateId>::Handled
            }
            _ => {
                //println!("Unhandled Event {}", event);
                EventAction::<StateId>::Unhandled
            }
        }
    }
}

fn main() {
    let mut actor = Actor { sm : StateMachine::default(), ctx : ActorCtx{} };
    actor.sm.register_top(StateId::TopState, Actor::top_state);
    actor.sm.register(StateId::State1, Actor::state1);
    actor.run()
}
