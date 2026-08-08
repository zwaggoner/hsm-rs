/// This module contains most of the core state machine traits and definitions
use core::marker::PhantomData;

/// `State` helper/wrapper type utilized throughout the framework as syntactic sugar for the a
/// reference to `StateDesc`
pub type State<Sm> = &'static StateDesc<Sm>;

pub(crate) const DEFAULT_MAX_NEST_DEPTH: usize = const {
    if let Some(depth) = option_env!("RSM_MAX_NEST_DEPTH") {
        const_str::parse!(depth, usize)
    } else {
        8
    }
};

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
///
/// impl StateMachineDef for MyActor {
///     type Event = MyEvent;
///
///     fn initial(&mut self) -> State<Self> {
///         State1::state()
///     }
/// }
///
/// # struct State1;
/// # impl StateDef<State1> for MyActor {
/// #    type Parent = Top;
/// #
/// # }
/// ```
/// State1 declaration is omitted here for brevity
///
/// Users can optionally specify `MAX_NEST_DEPTH`, operationally, `MAX_NEST_DEPTH` is used for
/// compile-time depth assertions on the depth of the state machine. Should [generic-const-exprs]
/// ever become stable rust, the intent of this field is to bound the maximum runtime storage needed
/// for calculating transition paths. Until this feature is stabilized though, users can manipulate
/// `DEFAULT_MAX_NEST_DEPTH` by setting the environment variable `RSM_MAX_NEST_DEPTH`.
/// `DEFAULT_MAX_NEST_DEPTH` is currently used as the bound for the runtime storage, which is
/// defaulted to 8,  and the trait impl of `MAX_NEST_DEPTH` is used for the runtime check. This choice was made to stabilize the
/// trait definition, while providing a clean deprecation path, despite the potential for divergence
/// between the two depth definitions.
pub trait StateMachineDef: Sized {
    /// Event type
    type Event: 'static;

    /// Depth Specification
    const MAX_NEST_DEPTH: usize = DEFAULT_MAX_NEST_DEPTH;

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
}

/// Sealed trait that provides the runtime glue for the parent tree
pub trait ParentState<Sm: StateMachineDef + 'static>: _private::Sealed {
    const OPT_STATE: Option<State<Sm>>;
    const DEPTH: usize;
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

const fn get_depth<Sm: StateDef<S> + 'static, S: 'static + _private::StaticStateDesc<Sm>>() -> usize
{
    let depth = <Sm as StateDef<S>>::Parent::DEPTH + 1;
    assert!(
        depth <= Sm::MAX_NEST_DEPTH,
        "Depth of state has exceeded the configured MAX_NEST_DEPTH"
    );

    depth
}

impl<Sm: StateDef<S> + 'static, S: 'static + _private::StaticStateDesc<Sm>> ParentState<Sm>
    for Super<S>
{
    const OPT_STATE: Option<State<Sm>> = Some(&S::STATE);
    const DEPTH: usize = get_depth::<Sm, S>();
}

impl<Sm: StateMachineDef + 'static> ParentState<Sm> for Top {
    const OPT_STATE: Option<State<Sm>> = None;
    const DEPTH: usize = 0;
}

/// Trait that provides a convenience wrapper for getting the runtime state descriptor object
pub trait StateRef<Sm: StateMachineDef + 'static>: _private::StaticStateDesc<Sm> {
    fn state() -> State<Sm>;
}

impl<S: 'static, Sm: StateDef<S> + 'static> _private::StaticStateDesc<Sm> for S {
    const STATE: StateDesc<Sm> = StateDesc::<Sm> {
        id: core::any::TypeId::of::<(Sm, S)>(),
        depth: get_depth::<Sm, S>(),
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
