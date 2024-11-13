pub trait StateHandlers<'a> {
    fn entry(&self) -> Option<&'a State<'a>>;
    //fn event<'b>(&self, evt: &'b dyn Event) -> Option<&'a State<'a>>;
    fn event(&self) -> Option<&'a State<'a>>;
    fn exit(&self) -> Option<&'a State<'a>>;
}

pub struct State<'a> {
    pub child: Option<&'a State<'a>>,
    pub handlers: &'a (dyn StateHandlers<'a> + Sync)
}

pub trait Event { }

pub struct QueueableEvent<'a> {
    next: Option<&'a mut QueueableEvent<'a>>,
    event: &'a (dyn Event + Sync)
}

impl<'a> QueueableEvent<'a> {
    pub fn new(event: &'a (dyn Event + Sync)) -> Self {
        Self { next: None, event: event }
    }
}

pub struct StateMachine<'a> {
    state: &'a State<'a>,
    event_queue: Option<&'a mut QueueableEvent<'a>>,
    init: bool
}

impl<'a> StateMachine<'a> {
    pub fn new(state: &'a State<'a>) -> Self {
        Self { state: state, event_queue: None, init: false }
    }

    pub fn execute(&mut self) { 
        if !self.init {
            self.state.handlers.entry();

            let mut curr_state = self.state;

            while let Some(child) = curr_state.child {
                child.handlers.entry();
                curr_state = child;
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

    pub fn post_event(&mut self, event: &'a mut QueueableEvent<'a>) {
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

