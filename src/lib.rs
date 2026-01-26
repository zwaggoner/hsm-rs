pub enum Event<Et> {
    Entry,
    Exit,
    Timeout,
    External(Et)
}

pub enum EventAction<Sid> {
    Unhandled,
    Handled,
    Transition(Sid)
}

pub trait StateId : Copy + Default {
    fn index(self) -> usize;
}

#[derive(Copy, Clone)]
struct StateInfo<C, Et, Sid : StateId> {
    child : Option<Sid>,
    handler : fn(&mut C, Event<Et>) -> EventAction<Sid>
}

pub struct StateMachine<C, Et, Sid : StateId, const N : usize> {
    state_table : [StateInfo<C, Et, Sid>; N],
    top : Sid
}

fn default_handler<C, Et, Sid>(_: &mut C, _: Event<Et>) -> EventAction<Sid> {
    EventAction::<Sid>::Unhandled
}

impl<C, Et, Sid : StateId, const N : usize> StateMachine<C, Et, Sid, N> {
    pub fn default() -> Self {
        Self {
            state_table : core::array::from_fn(|_| StateInfo { child : None, handler: default_handler}),
            top : Sid::default()
        }
    }

    pub fn register(&mut self, state_id : Sid, handler : fn(&mut C, Event<Et>) -> EventAction<Sid>) {
        self.state_table[state_id.index()] = StateInfo { child : None, handler : handler }; 
    }

    pub fn register_top(&mut self, state_id : Sid, handler : fn(&mut C, Event<Et>) -> EventAction<Sid>) {
        self.register(state_id, handler);
        self.top = state_id
    }

    fn exit_state(&mut self, context : &mut C, state: Sid) {
        if let Some(child) = self.state_table[state.index()].child { 
            self.exit_state(context, child);
            self.state_table[state.index()].child = None
        }

        _ = (self.state_table[state.index()].handler)(context, Event::Exit);
    }

    fn transition(&mut self, context: &mut C, state : Sid, new_child: Sid) {
        if let Some(child) = self.state_table[state.index()].child {
            self.exit_state(context, child)
        }

        self.enter_state(context, new_child)
    }

    fn enter_state(&mut self, context : &mut C, state: Sid) {
        let result = (self.state_table[state.index()].handler)(context, Event::Entry);

        if let EventAction::Transition(child) = result {
            self.state_table[state.index()].child = Some(child);
            self.enter_state(context, child);
        }
    }

    pub fn run(&mut self, context : &mut C) {
        self.enter_state(context, self.top)
    }
}
