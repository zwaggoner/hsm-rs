/// This module contains most of the core state machine traits and definitions
use core::marker::PhantomData;
use crate::util::fixed_vec::FixedVec;

/// `State` helper/wrapper type utilized throughout the framework as syntactic sugar for the a
/// reference to `StateDesc`
pub type State<Sm> = &'static StateDesc<Sm>;

pub struct Depth<Sm: StateMachineDef, const MAX_DEPTH: usize> {
    _pd: PhantomData<Sm>,
}

impl<Sm: StateMachineDef + 'static, const MAX_DEPTH: usize> _private::StatePath<Sm> for Depth<Sm, MAX_DEPTH> {
    type Storage = FixedVec<State<Sm>, MAX_DEPTH>;
    const MAX_DEPTH: usize = MAX_DEPTH;
}

/// The `StateMachineDef` trait is to be implemented by the user of the framework for any type
/// that the user wishes to implement a state machine to manage it. The type that the user
/// implements `StateMachineDef` can be thought of as the "context" object for all states in the
/// state machine if used as a state machine framework, or the object for which you are
/// implementing an actor if using the rsm framework as an actor framework. Users are required to
/// specify:
/// - `Event` Type
/// - Initial transition via the `initial` method
///
/// For example
/// ```
/// use rsm::*;
///
/// enum MyEvent {
///     Event1,
///     Event2,
/// }
///
/// struct MyActor;
/// struct State1;
///
/// impl StateMachineDef for MyActor {
///     type Event = MyEvent;
///
///     fn initial(&mut self) -> State<Self> {
///         State1::state()
///     }
/// }
///
/// # impl StateDef<State1> for MyActor {
/// #    type Parent = Top;
/// #
/// # }
/// ```
/// State1 declaration is omitted here for brevity
pub trait StateMachineDef: Sized {
    /// Event type
    type Event: 'static;

    /// Depth Specification
    type MaxDepth: _private::StatePath<Self>;

    /// Overall state machine initial transition (executed exactly once per state machine).
    fn initial(&mut self) -> State<Self>;
}

/// Action enum indicating how the state handler is responding to an event
pub enum Action<Sm: StateMachineDef + 'static> {
    /// The event was unhandled by the handler. This is largely used internal to the framework,
    /// but can be useful in cases where a state wants to know about an event, but also wants its
    /// parent to be able to handle it, or in match arms where certain events will be ignored by
    /// certain states.
    Unhandled,
    /// Indicates the event was handled by the handler. This stops propagation of the event to parent states as
    /// this means the event was "dealt with".  
    Handled,
    /// Indicates the event was handled by the handler, and the response to the event requires a
    /// transition to a new state as indicated by the `State` value in `Transition`
    Transition(State<Sm>),
}

/// The `StateDef` trait is to be implemented for all states in the state machine. It is
/// suggested/recommended that the State objects be zero-sized structs i.e.:
/// ```
/// struct State1;
/// ```
/// The state objects are never directly instantiated, nor are their contents available anywhere
/// else in the framework, and for program clarity it is not suggested to dual-purpose data
/// containing structs you otherwise utilize elsewhere.
///
/// `StateDef` requires that you have implemented `StateMachineDef`
///
/// Continuing our example from above:
///
/// ```
/// # use rsm::*;
/// #
/// # enum MyEvent {
/// #    Event1,
/// #    Event2,
/// #    Event3
/// # }
/// #
/// # struct MyActor;
/// #
/// # impl StateMachineDef for MyActor {
/// #    type Event = MyEvent;
/// #
/// #    fn initial(&mut self) -> State<Self> {
/// #        State1::state()
/// #    }
/// # }
/// # struct State2;
/// #
/// # impl StateDef<State2> for MyActor {
/// #   type Parent = Top;
/// # }
///
/// struct State1;
///
/// impl StateDef<State1> for MyActor {
///     // Use the Top type to signify this state has no parent, that is it is the topmost state
///     type Parent = Top;
///
///     // Perform entry actions occurs whenever the state is entered (transitioned to)
///     fn entry(&mut self) {
///         println!("My entry action");
///     }
///
///     // Handle any event dispatched to the state and indicate how it was handled via the Action enum
///     fn handler(&mut self, event: &MyEvent) -> Action<Self> {
///         match event {
///             MyEvent::Event1 => Action::Handled,
///             MyEvent::Event2 => Action::Unhandled,
///             MyEvent::Event3 => Action::Transition(State2::state()),
///         }
///     }
///
///     // Perform exit actions, occurs whenever the state is exited (transitioned away from)
///     fn exit(&mut self) {
///         println!("My exit action");
///     }
/// }
/// ```
///
pub trait StateDef<S>: StateMachineDef + Sized
where
    Self: 'static,
{
    type Parent: ParentState<Self>;

    fn initial(&mut self) -> Option<State<Self>> {
        None
    }

    fn entry(&mut self) {}

    fn handler(&mut self, _event: &Self::Event) -> Action<Self> {
        Action::Unhandled
    }

    fn exit(&mut self) {}
}

mod _private {
    use super::{StateDesc, StateMachineDef};

    pub trait Sealed {}
    pub trait StaticStateDesc<Sm: StateMachineDef + 'static> {
        const STATE: StateDesc<Sm>;
    }

    pub trait StatePath<Sm: StateMachineDef> {
        type Storage;
        const MAX_DEPTH: usize;
    }
}

/// Sealed trait that provides the runtime glue for the parent tree
pub trait ParentState<Sm: StateMachineDef + 'static>: _private::Sealed {
    const OPT_STATE: Option<State<Sm>>;
    const OPT_DEPTH: Option<usize>;
}

/// Type used to indicate the Parent of a given state in the `StateDef` declaration for example:
///
/// ```
/// # use rsm::*;
/// #
/// # enum MyEvent {
/// #    Event1,
/// # }
/// #
/// # struct MyActor;
/// #
/// # impl StateMachineDef for MyActor {
/// #    type Event = MyEvent;
/// #
/// #    fn initial(&mut self) -> State<Self> {
/// #        State1::state()
/// #    }
/// # }
/// # struct State2;
/// #
/// # impl StateDef<State2> for MyActor {
/// #   type Parent = Top;
/// # }
/// # struct State1;
/// #
/// # impl StateDef<State1> for MyActor {
///     // State2 is the parent state of State1
///     type Parent = Super<State2>;
/// # }
/// ```
pub struct Super<S>(PhantomData<S>);

/// Type used to indicate that a state has no parents, that is the state is a top-level state in
/// the state machine.
pub struct Top;

impl<S> _private::Sealed for Super<S> {}
impl _private::Sealed for Top {}

const fn next_depth(curr_opt_depth: Option<usize>) -> Option<usize> {
    if let Some(curr_depth) = curr_opt_depth {
        Some(curr_depth + 1)
    } else {
        Some(0usize)
    }
}

impl<Sm: StateDef<S> + 'static, S: 'static + _private::StaticStateDesc<Sm>> ParentState<Sm>
    for Super<S>
{
    const OPT_STATE: Option<State<Sm>> = Some(&S::STATE);
    const OPT_DEPTH: Option<usize> = next_depth(<Sm as StateDef<S>>::Parent::OPT_DEPTH);
}

impl<Sm: StateMachineDef + 'static> ParentState<Sm> for Top {
    const OPT_STATE: Option<State<Sm>> = None;
    const OPT_DEPTH: Option<usize> = None;
}

/// Trait that provides a convenience wrapper for getting the runtime state descriptor object
pub trait StateRef<Sm: StateMachineDef + 'static>: _private::StaticStateDesc<Sm> {
    fn state() -> State<Sm>;
}

impl<S: 'static, Sm: StateDef<S> + 'static> _private::StaticStateDesc<Sm> for S {
    const STATE: StateDesc<Sm> = StateDesc::<Sm> {
        id: core::any::TypeId::of::<(Sm, S)>(),
        depth: next_depth(Sm::Parent::OPT_DEPTH).unwrap(),
        parent: Sm::Parent::OPT_STATE,
        initial: <Sm as StateDef<S>>::initial,
        entry: Sm::entry,
        handler: Sm::handler,
        exit: Sm::exit,
    };
}

impl<S: 'static + _private::StaticStateDesc<Sm>, Sm: StateMachineDef + 'static> StateRef<Sm> for S {
    fn state() -> State<Sm> {
        &Self::STATE
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct StateDesc<Sm: StateMachineDef + 'static> {
    id: core::any::TypeId,
    pub(crate) depth: usize,
    pub(crate) parent: Option<State<Sm>>,
    pub(crate) initial: fn(&mut Sm) -> Option<State<Sm>>,
    pub(crate) entry: fn(&mut Sm),
    pub(crate) handler: fn(&mut Sm, &Sm::Event) -> Action<Sm>,
    pub(crate) exit: fn(&mut Sm),
}

impl<Sm: StateMachineDef> PartialEq for StateDesc<Sm> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
