//#![no_std]

#[macro_export]
macro_rules! state {
    (
        impl $name:ident {
            $($body:tt)*
        }
    ) => {
        struct $name;

        impl $crate::HsmState for $name {
            $($body)*
        }
    };
    (runtime $s:ty) => {
        &<$s as $crate::RuntimeState>::STATE
    };
}

type _State<C, E> = &'static StateDesc<C, E>;
pub type State<S> = _State<<S as HsmState>::Context, <S as HsmState>::Event>;

pub enum Action<C: 'static + std::fmt::Debug, E: 'static + std::fmt::Debug> {
    Unhandled,
    Handled,
    Transition(_State<C, E>),
}

pub type StateAction<S> = Action<<S as HsmState>::Context, <S as HsmState>::Event>;

#[doc(hidden)]
#[derive(Debug)]
pub struct StateDesc<C: 'static + std::fmt::Debug, E: 'static + std::fmt::Debug> {
    type_id: core::any::TypeId,
    parent: Option<_State<C, E>>,
    initial: fn(&mut C) -> Option<_State<C, E>>,
    entry: fn(&mut C),
    handler: fn(&mut C, &E) -> Action<C, E>,
    exit: fn(&mut C),
}

pub trait HsmState {
    type Context: 'static + std::fmt::Debug;
    type Event: 'static + std::fmt::Debug;

    const PARENT: Option<State<Self>> = None;

    fn initial(_context: &mut Self::Context) -> Option<State<Self>> {
        None
    }

    fn entry(_context: &mut Self::Context) {}

    fn handler(_context: &mut Self::Context, _event: &Self::Event) -> StateAction<Self> {
        StateAction::<Self>::Unhandled
    }

    fn exit(_context: &mut Self::Context) {}
}

pub trait RuntimeState: HsmState {
    const STATE: StateDesc<Self::Context, Self::Event>;
}

impl<S: HsmState + 'static> RuntimeState for S {
    const STATE: StateDesc<S::Context, S::Event> = StateDesc::<S::Context, S::Event> {
        type_id: core::any::TypeId::of::<S>(),
        parent: S::PARENT,
        initial: S::initial,
        entry: S::entry,
        handler: S::handler,
        exit: S::exit,
    };
}

pub struct StateMachine<S : HsmState, const MAX_NEST_DEPTH: usize = 32> 
{
    path: [Option<State<S>>; MAX_NEST_DEPTH],
    curr_depth: usize,
}

impl<S : HsmState + 'static, const MAX_NEST_DEPTH: usize> StateMachine<S, MAX_NEST_DEPTH> {
    pub fn default() -> Self {
        Self {
            path: [None; MAX_NEST_DEPTH],
            curr_depth: 0,
        }
    }

    fn enter_from(&mut self, context: &mut S::Context, state: State<S>, start: usize) {
        // Traverse parents until we find a shared one, or we reach the top
        let mut new_depth = start;
        let mut curr_state = state;
        let mut reverse_path: [Option<State<S>>; MAX_NEST_DEPTH] = [None; MAX_NEST_DEPTH];

        reverse_path[new_depth] = Some(curr_state);
        new_depth += 1;

        while let Some(parent_state) = curr_state.parent {
            if self.path[..self.curr_depth]
                .iter()
                .any(|curr_state: &Option<State<S>>| {
                    curr_state.unwrap().type_id == parent_state.type_id
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

    fn exit_to(&mut self, context: &mut S::Context, end: usize) {
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

    pub fn dispatch(&mut self, context: &mut S::Context, event: &S::Event) {
        for depth in (0..self.curr_depth).rev() {
            if let Some(state) = self.path[depth] {
                match (state.handler)(context, event) {
                    StateAction::<S>::Handled => break,
                    StateAction::<S>::Transition(new_state) => {
                        if core::ptr::eq(state, new_state) {
                            // Exit and re-enter same state;
                            self.exit_to(context, depth);
                            self.enter_from(context, new_state, depth);
                        } else {
                            // Find shared parent (if any)
                            let mut curr_state = new_state;
                            let mut parent_state: Option<State<S>> = None;
                            let mut transition_depth = depth;

                            while let Some(new_parent_state) = curr_state.parent {
                                if let Some(shared_parent_depth) =
                                    self.path[..self.curr_depth].iter().position(|&state| {
                                        state.unwrap().type_id == new_parent_state.type_id
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

    pub fn run(&mut self, context: &mut S::Context) {
        self.enter_from(context, state!(runtime S), 0);
    }
}
