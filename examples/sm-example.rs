extern crate rsm;

struct State1 {}
struct State2 {}

struct TopState<'a> {
    s1 : rsm::SMState<'a>,
    s2 : rsm::SMState<'a>
}

impl<'a> rsm::State<'a> for State1 {
    fn entry(&'a mut self) -> Option<&'a mut rsm::SMState<'a>> {
        println!("S1 Entry");

        None
    }

    fn event(&'a mut self) -> Option<&'a mut rsm::SMState<'a>> {
        println!("S1 Event");

        None
    }

    fn exit(&'a mut self) -> Option<&'a mut rsm::SMState<'a>> {
        println!("S1 Exit");

        None
    }
}

impl<'a> rsm::State<'a> for State2 {
    fn entry(&'a mut self) -> Option<&'a mut rsm::SMState<'a>> {
        println!("S2 Entry");

        None
    }

    fn event(&'a mut self) -> Option<&'a mut rsm::SMState<'a>> {
        println!("S2 Event");

        None
    }

    fn exit(&'a mut self) -> Option<&'a mut rsm::SMState<'a>> {
        println!("S2 Exit");

        None
    }
}

impl<'a> rsm::State<'a> for TopState<'a> {
    fn entry(&'a mut self) -> Option<&'a mut rsm::SMState<'a>> {
        println!("TopState Entry");

        Some(&mut self.s1)
    }

    fn event(&'a mut self) -> Option<&'a mut rsm::SMState<'a>> {
        println!("TopState Event");

        Some(&mut self.s2)
    }

    fn exit(&'a mut self) -> Option<&'a mut rsm::SMState<'a>> {
        println!("TopState Exit");

        None
    }
}

struct Empty{}

fn main() {
    let mut state1: State1 = State1{};
    let mut state2: State2 = State2{};
    let mut top_state = TopState { s1 : rsm::SMState::new(&mut state1), s2: rsm::SMState::new(&mut state2) };
    let mut sm: rsm::StateMachine<Empty> = rsm::StateMachine::new(&mut top_state);

    sm.execute();
}
