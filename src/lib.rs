/*
#![no_std]
use core::ptr;
*/
use std::ptr;

#[derive(Clone, Copy)]
pub struct Parentified { _private : () }

pub struct Parent<C, Et : Copy> { 
    state: State<C, Et>
}

impl Parentified {
    fn default() -> Self {
        Self { _private : () }
    }

    pub fn parent<C, Et : Copy>(&self, state: State<C, Et>) -> Action::<C, Et> {
        Action::Parent(Parent { state : state})
    }
}

#[derive(Clone, Copy)]
pub struct Transitionable { _private : () }

pub struct Transition<C, Et : Copy> { 
    target_state: State<C, Et>
}

impl Transitionable {
    fn default() -> Self {
        Self { _private : () }
    }

    pub fn transition<C, Et : Copy>(&self, state: State<C, Et>) -> Action::<C, Et> {
        Action::Transition(Transition { target_state : state})
    }
}

#[derive(Clone, Copy)]
pub enum Event<Et: Copy> {
    Parent(Parentified),
    Initial(Transitionable),
    Entry,
    Exit,
    Other{ event: Et, action : Transitionable},
}

pub enum Action<C, Et: Copy> {
    Parent(Parent<C, Et>),
    Unhandled,
    Handled,
    Transition(Transition<C, Et>),
}

type State<C, Et> = fn(&mut C, Event<Et>) -> Action<C, Et>;

pub struct StateMachine<C, Et: Copy, const MAX_NEST_DEPTH: usize = 32> {
    path: [Option<State<C, Et>>; MAX_NEST_DEPTH],
    curr_depth: usize,
}

impl<C, Et: Copy, const MAX_NEST_DEPTH: usize> StateMachine<C, Et, MAX_NEST_DEPTH> {
    pub fn default() -> Self {
        Self {
            path: [None; MAX_NEST_DEPTH],
            curr_depth: 0,
        }
    }

    pub fn initial(&mut self, context: &mut C, state: State<C, Et>) {
        self.enter_from(context, state, 0);
    }

    fn enter_from(&mut self, context: &mut C, state: State<C, Et>, start: usize) {
        // Take initial transitions until we find the bottom-most initial state
        let mut initial_state = state;

        while let Action::Transition(Transition { target_state : new_state }) = (initial_state)(context, Event::Initial(Transitionable::default())) {
            initial_state = new_state;
        }

        // Traverse parents until we find a shared one, or we reach the top
        self.curr_depth = start;
        let mut curr_state = initial_state;
        let mut reverse_path: [Option<State<C, Et>>; MAX_NEST_DEPTH] = [None; MAX_NEST_DEPTH];

        reverse_path[self.curr_depth] = Some(curr_state);
        self.curr_depth += 1;

        while let Action::Parent(Parent { state : parent_state}) = (curr_state)(context, Event::<Et>::Parent(Parentified::default())) {
            if self.path[..self.curr_depth].contains(&Some(parent_state)) {
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
                (state)(context, Event::Entry);
            }
        }
    }

    fn exit_to(&mut self, context: &mut C, end: usize) {
        for depth in (end..self.curr_depth).rev() {
            if let Some(state) = self.path[depth] {
                (state)(context, Event::Exit);

                if self.curr_depth > 0 {
                    self.curr_depth -= 1;
                } else {
                    break;
                }
            }
        }
    }

    pub fn dispatch(&mut self, context: &mut C, event: Et) {
        for depth in (0..self.curr_depth).rev() {
            if let Some(state) = self.path[depth] {
                match (state)(context, Event::Other{ event: event, action : Transitionable::default() }) {
                    Action::<C, Et>::Handled => break,
                    Action::<C, Et>::Transition(Transition { target_state : new_state }) => {
                        if ptr::fn_addr_eq(state, new_state) {
                            // Exit and re-enter same state;
                            self.exit_to(context, depth);
                            self.enter_from(context, new_state, depth);
                        } else {
                            // Find shared parent (if any)
                            let mut curr_state = new_state;
                            let mut parent_state: Option<State<C, Et>> = None;
                            let mut transition_depth = depth;

                            while let Action::Parent(Parent { state : new_parent_state}) =
                                (curr_state)(context, Event::<Et>::Parent(Parentified::default()))
                            {
                                if let Some(shared_parent_depth) =
                                    self.path[..self.curr_depth].iter().position(|&state| {
                                        ptr::fn_addr_eq(state.unwrap(), new_parent_state)
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

    pub fn run(&mut self, _context: &mut C) {}
}
