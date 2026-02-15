#![no_std]

#[macro_export]
macro_rules! state {
    (
        impl $name:ident {
            $($body:tt)*
        }
    ) => {
        struct $name;

        impl rsm::StateImpl for $name {
            $($body)*
        }
    };
    (runtime $s:ty) => {
        &<$s as rsm::RuntimeState>::STATE
    };
}

pub type State<C, E> = &'static StateDesc<C, E>;

pub enum Action<C: 'static, E: 'static> {
    Unhandled,
    Handled,
    Transition(State<C, E>),
}

#[derive(Debug)]
pub struct StateDesc<C: 'static, E: 'static> {
   parent: Option<State<C, E>>,
    initial: fn(&mut C) -> Option<State<C, E>>,
    entry: fn(&mut C),
    handler: fn(&mut C, &E) -> Action<C, E>,
    exit: fn(&mut C),
}

pub trait StateImpl {
    type Context: 'static;
    type Event: 'static;

    const PARENT: Option<State<Self::Context, Self::Event>> = None;

    fn initial(_context: &mut Self::Context) -> Option<State<Self::Context, Self::Event>> {
        None
    }

    fn entry(_context: &mut Self::Context) {}

    fn handler(
        _context: &mut Self::Context,
        _event: &Self::Event,
    ) -> Action<Self::Context, Self::Event> {
        Action::Unhandled
    }

    fn exit(_context: &mut Self::Context) {}
}

pub trait RuntimeState: StateImpl {
    const STATE: StateDesc<Self::Context, Self::Event>;
}

impl<S: StateImpl> RuntimeState for S {
    const STATE: StateDesc<S::Context, S::Event> = StateDesc::<S::Context, S::Event> {
        parent: S::PARENT,
        initial: S::initial,
        entry: S::entry,
        handler: S::handler,
        exit: S::exit,
    };
}

pub struct StateMachine<C: 'static, E: 'static, const MAX_NEST_DEPTH: usize = 32> {
    path: [Option<State<C, E>>; MAX_NEST_DEPTH],
    curr_depth: usize,
}

impl<C, E, const MAX_NEST_DEPTH: usize> StateMachine<C, E, MAX_NEST_DEPTH> {
    pub fn default() -> Self {
        Self {
            path: [None; MAX_NEST_DEPTH],
            curr_depth: 0,
        }
    }

    pub fn initial(&mut self, context: &mut C, state: State<C, E>) {
        self.enter_from(context, state, 0);
    }

    fn enter_from(&mut self, context: &mut C, state: State<C, E>, start: usize) {
        // Traverse parents until we find a shared one, or we reach the top
        let mut new_depth = start;
        let mut curr_state = state;
        let mut reverse_path: [Option<State<C, E>>; MAX_NEST_DEPTH] = [None; MAX_NEST_DEPTH];

        reverse_path[new_depth] = Some(curr_state);
        new_depth += 1;

        while let Some(parent_state) = curr_state.parent {
            if self.path[..self.curr_depth]
                .iter()
                .any(|curr_state: &Option<State<C, E>>| {
                    core::ptr::eq(curr_state.unwrap(), parent_state)
                })
            {
                break;
            } else {
                reverse_path[new_depth] = Some(parent_state);
                curr_state = parent_state;
                new_depth += 1;
            }
        }

        for (idx, state_opt) in (&reverse_path[start..new_depth]).iter().enumerate() {
            self.path[new_depth - idx - 1] = *state_opt;
        }

        self.curr_depth = new_depth;

        for state_opt in &self.path[start..self.curr_depth] {
            if let Some(state) = state_opt {
                (state.entry)(context);
            }
        }

        let leaf_state = self.path[self.curr_depth - 1].unwrap();

        if let Some(initial_state) = (leaf_state.initial)(context) {
            self.enter_from(context, initial_state, self.curr_depth);
        }
    }

    fn exit_to(&mut self, context: &mut C, end: usize) {
        for depth in (end..self.curr_depth).rev() {
            if let Some(state) = self.path[depth] {
                (state.exit)(context);
                self.path[depth] = None;

                if self.curr_depth > 0 {
                    self.curr_depth -= 1;
                } else {
                    break;
                }
            }
        }
    }

    pub fn dispatch(&mut self, context: &mut C, event: &E) {
        for depth in (0..self.curr_depth).rev() {
            if let Some(state) = self.path[depth] {
                match (state.handler)(context, event) {
                    Action::<C, E>::Handled => break,
                    Action::<C, E>::Transition(new_state) => {
                        if core::ptr::eq(state, new_state) {
                            // Exit and re-enter same state;
                            self.exit_to(context, depth);
                            self.enter_from(context, new_state, depth);
                        } else {
                            // Find shared parent (if any)
                            let mut curr_state = new_state;
                            let mut parent_state: Option<State<C, E>> = None;
                            let mut transition_depth = depth;

                            while let Some(new_parent_state) = curr_state.parent {
                                if let Some(shared_parent_depth) =
                                    self.path[..self.curr_depth].iter().position(|&state| {
                                        core::ptr::eq(state.unwrap(), new_parent_state)
                                    })
                                {
                                    transition_depth = shared_parent_depth + 1;
                                    parent_state = Some(new_parent_state);
                                    break;
                                } else {
                                    curr_state = new_parent_state;
                                }
                            }

                            if let Some(_shared_parent) = parent_state {
                                // Exit up to the shared parent, then enter down from the new state
                                self.exit_to(context, transition_depth);
                                self.enter_from(context, new_state, transition_depth);
                            } else {
                                // We reached the top with no shared parent, top state changed
                                self.exit_to(context, 0);
                                self.enter_from(context, new_state, 0);
                            }
                        }
                        break;
                    }
                    _ => continue,
                }
            }
        }
    }

    //pub fn run(&mut self, _context: &mut C) {}
}
