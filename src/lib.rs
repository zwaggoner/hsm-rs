#![no_std]

#[macro_export]
macro_rules! state {
    (
        impl $name:ident$(::)?<$context:ident, $event:ident> {
            $($body:tt)*
        }
    ) => {
        struct $name;

        impl rsm::StateImpl<$context, $event> for $name {
            $($body)*
        }
    };
    (
        impl $name:ident$(::)?<$context:ident, $event:ident> : $parent:ident {
            $($body:tt)*
        }
    ) => {
        state!{
            impl $name<$context, $event> {
                const PARENT : Option<rsm::State::<$context, $event>> = Some(state!(runtime $parent<$context, $event>));

                $($body)*
            }
        }
    };
    (runtime $s:ident$(::)?<$context:ident, $event:ident>) => {
        &<$s as rsm::RuntimeState::<$context, $event>>::STATE
    };
}

pub type State<C, E> = &'static StateDesc<C, E>;

pub enum Action<C : 'static, E : 'static> {
    Unhandled,
    Handled,
    Transition(State::<C, E>)
}

pub struct StateDesc<C : 'static, E : 'static> {
    parent : Option<State::<C, E>>,
    initial: fn(&mut C) -> Option<State::<C, E>>,
    entry: fn(&mut C),
    handler: fn(&mut C, E) -> Action<C, E>,
    exit : fn(&mut C),
}

pub trait StateImpl<C : 'static, E : 'static> {
    const PARENT : Option<State::<C, E>> = None;

    fn initial(_context : &mut C) -> Option<State::<C, E>> {
        None
    }

    fn entry(_context : &mut C) { }

    fn handler(_context : &mut C, _event : E) -> Action<C, E> {
        Action::Unhandled
    }

    fn exit(_context : &mut C) { }
}

pub trait RuntimeState<C : 'static, E : 'static> : StateImpl<C, E> {
    const STATE: StateDesc::<C, E>;
}

impl<S : StateImpl<C, E>, C : 'static, E : 'static> RuntimeState<C, E> for S
{
    const STATE: StateDesc::<C, E> = StateDesc::<C, E> {
        parent: S::PARENT,
        initial: S::initial,
        entry : S::entry,
        handler : S::handler,
        exit: S::exit,
    };
}

pub struct StateMachine<C : 'static, E: 'static, const MAX_NEST_DEPTH: usize = 32> {
    path: [Option<State<C, E>>; MAX_NEST_DEPTH],
    curr_depth: usize,
}

impl<C, E: Copy, const MAX_NEST_DEPTH: usize> StateMachine<C, E, MAX_NEST_DEPTH> {
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
        let mut initial_state = state;

        // Check for initial transition
        if let Some(new_initial_state) = (state.initial)(context) {
            initial_state = new_initial_state;
        }

        // Traverse parents until we find a shared one, or we reach the top
        self.curr_depth = start;
        let mut curr_state = initial_state;
        let mut reverse_path: [Option<State<C, E>>; MAX_NEST_DEPTH] = [None; MAX_NEST_DEPTH];

        reverse_path[self.curr_depth] = Some(curr_state);
        self.curr_depth += 1;

        while let Some(parent_state) = curr_state.parent
        {
            if self.path[..self.curr_depth].iter().any(|curr_state : &Option<State<C, E>>| core::ptr::eq(curr_state.unwrap(), parent_state)) {
                break;
            } else {
                reverse_path[self.curr_depth] = Some(parent_state);
                curr_state = parent_state;
                self.curr_depth += 1;
            }
        }

        for (idx, state_opt) in (&reverse_path[start..self.curr_depth]).iter().enumerate() {
            self.path[self.curr_depth - idx - 1] = *state_opt;
        }

        for state_opt in &self.path[start..self.curr_depth] {
            if let Some(state) = state_opt {
                (state.entry)(context);
            }
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

    pub fn dispatch(&mut self, context: &mut C, event: E) {
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

                            while let Some(new_parent_state) = curr_state.parent
                            {
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
