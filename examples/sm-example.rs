extern crate rsm;

struct State1 {}
struct State2 {}

struct TopState<'a> {
    s1 : rsm::SMState<'a>,
    s2 : rsm::SMState<'a>
}

impl<'a> rsm::State<'a> for State1 {
    fn entry(&'a self) -> Option<&'a rsm::SMState<'a>> {
        println!("S1 Entry");

        None
    }

    fn event(&'a self) -> Option<&'a rsm::SMState<'a>> {
        println!("S1 Event");

        None
    }

    fn exit(&'a self) -> Option<&'a rsm::SMState<'a>> {
        println!("S1 Exit");

        None
    }
}

impl<'a> rsm::State<'a> for State2 {
    fn entry(&'a self) -> Option<&'a rsm::SMState<'a>> {
        println!("S2 Entry");

        None
    }

    fn event(&'a self) -> Option<&'a rsm::SMState<'a>> {
        println!("S2 Event");

        None
    }

    fn exit(&'a self) -> Option<&'a rsm::SMState<'a>> {
        println!("S2 Exit");

        None
    }
}

impl<'a> rsm::State<'a> for TopState<'a> {
    fn entry(&'a self) -> Option<&'a rsm::SMState<'a>> {
        println!("TopState Entry");

        Some(&self.s1)
    }

    fn event(&'a self) -> Option<&'a rsm::SMState<'a>> {
        println!("TopState Event");

        None
    }

    fn exit(&'a self) -> Option<&'a rsm::SMState<'a>> {
        println!("TopState Exit");

        None
    }
}

struct Event1 { }

impl rsm::Event for Event1 {}

fn main() {
    let state1: State1 = State1{};
    let state2: State2 = State2{};
    let top_state = TopState { s1 : rsm::SMState::new(&state1), s2: rsm::SMState::new(&state2) };
    let mut sm: rsm::StateMachine = rsm::StateMachine::new(&top_state);

    sm.execute();
}
