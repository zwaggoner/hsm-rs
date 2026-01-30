use std::cell::RefCell;

#[derive(Clone, Copy)]
pub enum Event<Et : Copy> {
    GetParent,
    Initial,
    Entry,
    Exit,
    Other(Et)
}

pub enum Action<C, Et : Copy> {
    Parent(State<C, Et>),
    Unhandled,
    Handled,
    Transition(State<C, Et>)
}

type State<C, Et : Copy> = fn(&mut C, Event<Et>) -> Action<C, Et>;

pub struct StateMachine<C, Et : Copy, const MAX_NEST_DEPTH : usize = 32> {
    path : RefCell<[Option<State<C, Et>>; MAX_NEST_DEPTH]>,
    curr_depth : RefCell<usize>
}

impl<C, Et : Copy, const MAX_NEST_DEPTH : usize> StateMachine<C, Et, MAX_NEST_DEPTH> {
    pub fn default() -> Self {
            Self { path : RefCell::new([None; MAX_NEST_DEPTH]), curr_depth: RefCell::new(0) }
    }

    pub fn initial(&self, context : &mut C, state : State<C, Et>) {
        self.enter_from(context, state, 0);
    }

    fn enter_from(&self, context : &mut C, state : State<C, Et>, start : usize) {
        // Take initial transitions until we find the bottom-most initial state
        let mut initial_state = state;

        while let Action::Transition(new_state) = (initial_state)(context, Event::Initial) {
            initial_state = new_state;
        }

        // Traverse parents until we find a shared one, or we reach the top
        let mut depth : usize = start;
        let mut curr_state = initial_state;
        let mut reverse_path : [Option<State<C, Et>>; MAX_NEST_DEPTH] = [None; MAX_NEST_DEPTH];

        reverse_path[depth] = Some(curr_state);
        depth += 1;

        while let Action::Parent(parent_state) = (curr_state)(context, Event::GetParent) {
            if self.path.borrow().contains(&Some(parent_state)) {
                break;
            }
            else {
                reverse_path[depth] = Some(parent_state);
                curr_state = parent_state;
                depth += 1;
            }
        }

        for state_opt in reverse_path {
            if let Some(state) = state_opt {
                depth -= 1;
                self.path.borrow_mut()[depth] = Some(state);
            }
        }

        while let Some(state) = self.path.borrow()[depth] {
            (state)(context, Event::Entry);
            depth += 1;                
        }
        
        *self.curr_depth.borrow_mut() = depth - 1;
    }

    fn exit_to(&self, context : &mut C, end : usize) {
        let mut curr_depth : usize = 0;

        {
            curr_depth = *self.curr_depth.borrow();
        }

        for depth in (end..=curr_depth).rev() {
            if let Some(state) = self.path.borrow()[depth] {
                (state)(context, Event::Exit);

                let mut curr_depth = self.curr_depth.borrow_mut();

                if *curr_depth > 0 {
                    *curr_depth -= 1;
                }
                else {
                    break;
                }
            }
        }
    }

    pub fn dispatch(&self, context: &mut C, event : Event<Et>) {
        let curr_depth : usize = *self.curr_depth.borrow();

        for depth in (0..=curr_depth).rev() {
            let mut curr_path : Option<State<C, Et>> = None;

            {
                curr_path = self.path.borrow()[depth]
            }

            if let Some(state) =  curr_path {
                match (state)(context, event) {
                    Action::<C, Et>::Handled => break,
                    Action::<C, Et>::Transition(new_state) => {
                        if state == new_state {
                            // Exit and re-enter same state;
                            self.exit_to(context, depth);
                            self.enter_from(context, new_state, depth);
                        } 
                        else {
                            // Find shared parent (if any)
                            let mut curr_state = new_state;
                            let mut parent_state : Option<State<C, Et>> = None;
                            let mut transition_depth = depth;

                            while let Action::Parent(new_parent_state) = (curr_state)(context, Event::GetParent) {
                                if let Some(shared_parent_depth) = self.path.borrow().iter().position(|&state| state == Some(new_parent_state)) {
                                    transition_depth = shared_parent_depth + 1;
                                    parent_state = Some(new_parent_state);
                                    break;
                                }
                                else {
                                    curr_state = new_parent_state;
                                }
                            }


                            if let Some(shared_parent) = parent_state {
                                self.exit_to(context, transition_depth);        
                                self.enter_from(context, new_state, transition_depth);
                            }
                            else {
                                println!("Hello?");
                                // We reached the top with no shared parent, top state changed
                                self.exit_to(context, 0);
                                self.enter_from(context, new_state, 0);
                            }
                        }
                        break;
                    },
                    _ => continue
                }
            }
        }
    }

    pub fn run(&self, context : &mut C) {
    }
}
