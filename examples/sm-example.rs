use std::cell::RefCell;

trait State {
    fn entry(&mut self) -> Option<&mut dyn State>; 
    fn on_event(&mut self) -> Option<&mut dyn State>;
}

struct StateMachine<'a> {
    top_state : &'a mut dyn State
} 

impl<'a> StateMachine<'a> {
    fn run(&mut self) {
        let mut parent_state : &mut dyn State = self.top_state;

        while let Some(child_state) = parent_state.entry() {
            parent_state = child_state;
        }
    }
}

struct State1 {}
struct State2 {}

impl State for State1 {
    fn entry(&mut self) -> Option<&mut dyn State> {
        println!("Entering State1");

        None
    }

    fn on_event(&mut self) -> Option<&mut dyn State> {
        println!("Got Event in State1");

        None
    }
}

impl State for State2 {
    fn entry(&mut self) -> Option<&mut dyn State> {
        println!("Entering State2");

        None
    }

    fn on_event(&mut self) -> Option<&mut dyn State> {
        println!("Got Event in State2");

        None
    }
}

struct TopState{
    s1 : State1,
    s2 : State2
}

impl State for TopState {
    fn entry(&mut self) -> Option<&mut dyn State> {
        println!("Entering TopState");

        Some(&mut self.s1)
    }

    fn on_event(&mut self) -> Option<&mut dyn State> {
        println!("Got Event in TopState");

        Some(&mut self.s2)
    }
}

fn main() {
    let mut ts =  TopState{s1: State1{}, s2: State2{}};
    let mut sm = StateMachine{ top_state: &mut ts };

    sm.run();
}
