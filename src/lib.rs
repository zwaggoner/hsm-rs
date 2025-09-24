pub enum SMEvent<T> {
    Timeout,
    Custom(T)
}

pub trait State {
    fn entry(&mut self) -> Option<&mut SMState>;
    fn event(&mut self) -> Option<&mut SMState>;
    fn exit(&mut self) -> Option<&mut SMState>;
}

pub struct SMState<'a> {
    child: Option<&'a mut SMState<'a>>,
    state: &'a mut (dyn State + Sync)
}

impl<'a> SMState<'a> {
    pub fn new(state: &'a mut (dyn State + Sync)) -> Self {
        Self { state: state, child: None }
    }
}

pub struct QueueableEvent<'a, T> {
    next: Option<&'a mut QueueableEvent<'a, T>>,
    event: SMEvent<T>
}

impl<'a, T> QueueableEvent<'a, T> {
    pub fn new(event: SMEvent<T>) -> Self {
        Self { next: None, event: event }
    }
}

pub struct StateMachine<'a, 'b, T> {
    top_state: SMState<'a>,
    event_queue: Option<&'b mut QueueableEvent<'b, T>>,
    init: bool
}

impl<'a, 'b, T> StateMachine<'a, 'b, T> {
    pub fn new(top_state: &'a mut (dyn State + Sync)) -> Self {
        Self { top_state: SMState::new(top_state), event_queue: None, init: false }
    }

    pub fn execute(&mut self) { 
        if !self.init {
            let mut curr_state : &mut SMState = &mut self.top_state;

            while let Some(child) = curr_state.state.entry() { curr_state.child = Some(child);
                curr_state = curr_state.child.as_mut().unwrap();
            }
        }

//        while let Some(curr_event) = self.event_queue {
//            let mut curr_state = self.state;
//
//            while let Some(child) = curr_state.child {
//                if let Some(next_state) = child.handlers.event(curr_event.event) {
//                    child.handlers.exit();
//                    child = next_state;
//                    child.handlers.entry();
//
//                    let mut new_state = child;
//
//                    while let Some(substate) = new_state.child {
//                        substate.handlers.entry();
//                        new_state = substate;
//                    }
//                    break;
//                }
//
//                curr_state = child;
//            }
//
//            self.event_queue = Some(self.event_queue.as_mut().expect(""));
//        }
    }

    pub fn post_event(&mut self, event: &'b mut QueueableEvent<T>) {
        if let Some(mut last_event) = self.event_queue.as_mut() {
            while let Some(_)  = last_event.next {
                last_event = last_event.next.as_mut().expect("");
            }

            last_event.next = Some(event);
        }
        else {
            self.event_queue = Some(event);
        }
    }
}

