/// This module contains most of the core state machine traits and definitions
use core::marker::PhantomData;

/// `State` helper/wrapper type utilized throughout the framework as syntactic sugar for the a
/// reference to `StateDesc`
pub type State<Sm> = &'static StateDesc<Sm>;

/// The `StateMachineSpec` trait is to be implemented by the user of the framework for any type
/// that the user wishes to implement a state machine to manage it. The type that the user
/// implements `StateMachineSpec` can be thought of as the "context" object for all states in the
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
/// impl StateMachineSpec for MyActor {
///     type Event = MyEvent;
///
///     fn initial(&mut self) -> State<Self> {
///         State1::state()
///     }
/// }
///
/// # impl StateImpl<State1> for MyActor {
/// #    type Parent = Root;
/// #
/// # }
/// ```
/// State1 declaration is omitted here for brevity
pub trait StateMachineSpec: Sized {
    /// Event type
    type Event: 'static;

    /// Overall state machine initial transition (executed exactly once per state machine).
    fn initial(&mut self) -> State<Self>;
}

/// Action enum indicating how the state handler is responding to an event
pub enum Action<Sm: StateMachineSpec + 'static> {
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

/// The `StateImpl` trait is to be implemented for all states in the state machine. It is
/// suggested/recommended that the State objects be zero-sized structs i.e.:
/// ```
/// struct State1;
/// ```
/// The state objects are never directly instantiated, nor are their contents available anywhere
/// else in the framework, and for program clarity it is not suggested to dual-purpose data
/// containing structs you otherwise utilize elsewhere.
///
/// `StateImpl` requires that you have implemented `StateMachineSpec`
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
/// # impl StateMachineSpec for MyActor {
/// #    type Event = MyEvent;
/// #
/// #    fn initial(&mut self) -> State<Self> {
/// #        State1::state()
/// #    }
/// # }
/// # struct State2;
/// #
/// # impl StateImpl<State2> for MyActor {
/// #   type Parent = Root;
/// # }
///
/// struct State1;
///
/// impl StateImpl<State1> for MyActor {
///     // Use the Root type to signify this state has no parent i.e. it is a topmost (root) state.
///     type Parent = Root;
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
pub trait StateImpl<S>: StateMachineSpec + Sized
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
    use super::{StateDesc, StateMachineSpec};

    pub trait Sealed {}
    pub trait StaticStateDesc<Sm: StateMachineSpec + 'static> {
        const STATE: StateDesc<Sm>;
    }
}

/// Sealed trait that provides the runtime glue for the parent tree
pub trait ParentState<Sm: StateMachineSpec + 'static>: _private::Sealed {
    const OPT_STATE: Option<State<Sm>>;
}

/// Type used to indicate the Parent of a given state in the `StateImpl` declaration for example:
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
/// # impl StateMachineSpec for MyActor {
/// #    type Event = MyEvent;
/// #
/// #    fn initial(&mut self) -> State<Self> {
/// #        State1::state()
/// #    }
/// # }
/// # struct State2;
/// #
/// # impl StateImpl<State2> for MyActor {
/// #   type Parent = Root;
/// # }
/// # struct State1;
/// # 
/// # impl StateImpl<State1> for MyActor {
///     // State2 is the parent state of State1
///     type Parent = Parent<State2>;
/// # }
/// ```
pub struct Parent<S>(PhantomData<S>);

/// Type used to indicate that a state has no parents, that is the state is a top-level state in
/// the state machine.
pub struct Root;

impl<S> _private::Sealed for Parent<S> {}
impl _private::Sealed for Root {}

impl<Sm: StateMachineSpec + 'static, S: 'static + _private::StaticStateDesc<Sm>> ParentState<Sm>
    for Parent<S>
{
    const OPT_STATE: Option<State<Sm>> = Some(&S::STATE);
}

impl<Sm: StateMachineSpec + 'static> ParentState<Sm> for Root {
    const OPT_STATE: Option<State<Sm>> = None;
}

/// Trait that provides a convenience wrapper for getting the runtime state descriptor object
pub trait StateRef<Sm: StateMachineSpec + 'static>: _private::StaticStateDesc<Sm> {
    fn state() -> State<Sm>;
}

impl<S: 'static, Sm: StateImpl<S> + 'static> _private::StaticStateDesc<Sm> for S {
    const STATE: StateDesc<Sm> = StateDesc::<Sm> {
        id: core::any::TypeId::of::<(Sm, S)>(),
        parent: Sm::Parent::OPT_STATE,
        initial: <Sm as StateImpl<S>>::initial,
        entry: Sm::entry,
        handler: Sm::handler,
        exit: Sm::exit,
    };
}

impl<S: 'static + _private::StaticStateDesc<Sm>, Sm: StateMachineSpec + 'static> StateRef<Sm>
    for S
{
    fn state() -> State<Sm> {
        &Self::STATE
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct StateDesc<Sm: StateMachineSpec + 'static> {
    id: core::any::TypeId,
    pub(crate) parent: Option<State<Sm>>,
    pub(crate) initial: fn(&mut Sm) -> Option<State<Sm>>,
    pub(crate) entry: fn(&mut Sm),
    pub(crate) handler: fn(&mut Sm, &Sm::Event) -> Action<Sm>,
    pub(crate) exit: fn(&mut Sm),
}

impl<Sm: StateMachineSpec> PartialEq for StateDesc<Sm> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
