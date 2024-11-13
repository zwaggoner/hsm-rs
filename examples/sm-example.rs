extern crate rsm;

struct State1Impl {}
struct State2Impl {}
struct State3Impl {}

impl<'a> rsm::StateHandlers<'a> for State1Impl {
    fn entry(&self) -> Option<&'a rsm::State<'a>> {
        println!("S1 Entry Handler");

        None
    }

    fn event(&self) -> Option<&'a rsm::State<'a>> {
        println!("S1 Event Handler");

        None
    }

    fn exit(&self) -> Option<&'a rsm::State<'a>> {
        println!("S1 Exit Handler");

        None
    }
}

impl<'a> rsm::StateHandlers<'a> for State2Impl {
    fn entry(&self) -> Option<&'a rsm::State<'a>> {
        println!("S2 Entry Handler");

        None
    }

    fn event(&self) -> Option<&'a rsm::State<'a>> {
        println!("S2 Event Handler");

        None
    }

    fn exit(&self) -> Option<&'a rsm::State<'a>> {
        println!("S2 Exit Handler");

        None
    }
}

impl<'a> rsm::StateHandlers<'a> for State3Impl {
    fn entry(&self) -> Option<&'a rsm::State<'a>> {
        println!("S3 Entry Handler");

        None
    }

    fn event(&self) -> Option<&'a rsm::State<'a>> {
        println!("S3 Event Handler");

        None
    }

    fn exit(&self) -> Option<&'a rsm::State<'a>> {
        println!("S3 Exit Handler");

        None
    }
}

struct Event1 { }

impl rsm::Event for Event1 {}

fn main() {
    let s1: State1Impl = State1Impl {};
    let s2: State2Impl = State2Impl {};
    let s3: State3Impl = State3Impl {};
    let state2: rsm::State = rsm::State{ child : None, handlers : &s2 };
    let state1: rsm::State = rsm::State{ child : Some(&state2), handlers : &s1 };
    let mut transition_event: rsm::QueueableEvent = rsm::QueueableEvent::new(&Event1{});
    let mut sm: rsm::StateMachine = rsm::StateMachine::new(&state1);

    sm.execute();
    sm.post_event(&mut transition_event);        
}
