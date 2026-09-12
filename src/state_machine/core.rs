/// This module contains most of the core state machine traits and definitions
use core::marker::PhantomData;

/// `State` object representing the runtime state descriptor in the framework
pub struct State<Sm: StateMachineDef + 'static> {
    desc: &'static StateDesc<Sm>,
}

impl<Sm: StateMachineDef + 'static> State<Sm> {
    const fn new(desc: &'static StateDesc<Sm>) -> Self {
        Self { desc }
    }

    pub(in crate::state_machine) const fn depth(self) -> usize {
        self.desc.depth
    }

    pub(in crate::state_machine) const fn parent(self) -> Option<Self> {
        self.desc.parent
    }

    pub(in crate::state_machine) fn initial(self, context: &mut Sm) -> Option<Self> {
        (self.desc.initial)(context)
    }

    pub(in crate::state_machine) fn entry(self, context: &mut Sm) {
        (self.desc.entry)(context);
    }

    pub(in crate::state_machine) fn handler(
        self,
        context: &mut Sm,
        event: &Sm::Event,
    ) -> Action<Sm> {
        (self.desc.handler)(context, event)
    }

    pub(in crate::state_machine) fn exit(self, context: &mut Sm) {
        (self.desc.exit)(context);
    }
}

impl<Sm: StateMachineDef + 'static> Clone for State<Sm> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Sm: StateMachineDef + 'static> Copy for State<Sm> {}

impl<Sm: StateMachineDef + 'static> PartialEq for State<Sm> {
    fn eq(&self, other: &Self) -> bool {
        self.desc == other.desc
    }
}

impl<Sm: StateMachineDef + 'static> Eq for State<Sm> {}

pub(in crate::state_machine) const DEFAULT_MAX_NEST_DEPTH: usize = const {
    if let Some(depth) = option_env!("HSM_RS_MAX_NEST_DEPTH") {
        const_str::parse!(depth, usize)
    } else {
        8
    }
};

/// The `StateMachineDef` trait is to be implemented by the user of the framework for any type
/// that the user wishes to implement a state machine to manage it. The type that the user
/// implements `StateMachineDef` can be thought of as the "context" object for all states in the
/// state machine if used as a state machine framework, or the object for which you are
/// implementing an actor if using the hsm-rs framework as an actor framework. Users are required to
/// specify:
/// - `Event` Type
/// - Initial transition via the `initial` method
///
/// For example
/// ```
/// use hsm_rs::*;
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
/// The maximum depth of nested states is controlled for all state machines by a private constant `DEFAULT_MAX_NEST_DEPTH`
/// which defaults to 8. This is used to check the nest depth of state machines at compile time,
/// as well as to set the temporary storage capacity for temporary path calculations during
/// transition. This is scratch space is stack allocated as a `FixedVec` of `State`s, and will be
/// allocated for each transition. The `DEFAULT_MAX_NEST_DEPTH` can be controlled by the user via
/// the environment variable `HSM_RS_MAX_NEST_DEPTH`.  
///
/// Should
/// [generic_const_exprs](https://doc.rust-lang.org/nightly/unstable-book/language-features/generic-const-exprs.html)
/// ever become stable rust, or one of the associated MVPs, the original design intent is that the user can control the nest depth on a
/// per state machine basis by setting `MAX_NEST_DEPTH`. However, as of this release, there was no
/// acceptable way in stable rust identified to achieve this design goal, so the environment variable override was
/// added so that the configuration setting can be customized/optimized by the user without
/// creating an otherwise undesirable API.
pub trait StateMachineDef: Sized {
    /// Event type
    type Event: 'static;

    /// Overall state machine initial transition (executed exactly once per state machine).
    fn initial(&mut self) -> State<Self>;

    // Future: Depth Specification
    // const MAX_NEST_DEPTH: usize = DEFAULT_MAX_NEST_DEPTH;
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
/// # use hsm_rs::*;
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
/// Note that it is only required to define the Parent state, all handlers default to an effective
/// no-op.
pub trait StateDef<S>: StateMachineDef + Sized
where
    Self: 'static,
{
    /// Define the parent as either `Top` if this is the topmost state or `Super<MyState>`
    type Parent: ParentState<Self>;

    /// Specify initial transition actions and return the target of the initial transition, which
    /// can be `None` if there is no initial transition.
    fn initial(&mut self) -> Option<State<Self>> {
        None
    }

    /// Define any entry actions
    fn entry(&mut self) {}

    /// Event handler for any events
    fn handler(&mut self, _event: &Self::Event) -> Action<Self> {
        Action::Unhandled
    }

    /// Define any exit actions
    fn exit(&mut self) {}
}

mod _private {
    pub trait SealedParentState {}
    pub trait SealedStateRef {}
}

trait StaticStateDesc<Sm: StateMachineDef + 'static> {
    const STATE: StateDesc<Sm>;
}

/// Sealed trait for `Super` and `Top` to limit definitions of `Parent` while defining the
/// `StateDef` for a given state.
pub trait ParentState<Sm: StateMachineDef + 'static>: _private::SealedParentState {
    const OPT_STATE: Option<State<Sm>>;
    const DEPTH: usize;
}

/// Type used to indicate the Parent of a given state in the `StateDef` declaration for example:
///
/// ```
/// # use hsm_rs::*;
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

impl<S> _private::SealedParentState for Super<S> {}
impl _private::SealedParentState for Top {}

const fn get_depth<Sm: StateDef<S> + 'static, S: 'static + StaticStateDesc<Sm>>() -> usize {
    let depth = <Sm as StateDef<S>>::Parent::DEPTH + 1;
    assert!(
        depth <= DEFAULT_MAX_NEST_DEPTH,
        "Depth of state has exceeded the configured DEFAULT_MAX_NEST_DEPTH"
    );

    depth
}

impl<Sm: StateDef<S> + 'static, S: 'static + StaticStateDesc<Sm>> ParentState<Sm> for Super<S> {
    const OPT_STATE: Option<State<Sm>> = Some(State::new(&S::STATE));
    const DEPTH: usize = get_depth::<Sm, S>();
}

impl<Sm: StateMachineDef + 'static> ParentState<Sm> for Top {
    const OPT_STATE: Option<State<Sm>> = None;
    const DEPTH: usize = 0;
}

/// Sealed Trait automatically implemented for each `StateDef` that maps the state object to the runtime `State` object
pub trait StateRef<Sm: StateMachineDef + 'static>: _private::SealedStateRef {
    /// Returns the runtime state descriptor for the state
    fn state() -> State<Sm>;
}

impl<S: 'static> _private::SealedStateRef for S {}

impl<S: 'static, Sm: StateDef<S> + 'static> StaticStateDesc<Sm> for S {
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

impl<S: 'static + StaticStateDesc<Sm> + _private::SealedStateRef, Sm: StateMachineDef + 'static>
    StateRef<Sm> for S
{
    fn state() -> State<Sm> {
        State::new(&Self::STATE)
    }
}

struct StateDesc<Sm: StateMachineDef + 'static> {
    id: core::any::TypeId,
    depth: usize,
    parent: Option<State<Sm>>,
    initial: fn(&mut Sm) -> Option<State<Sm>>,
    entry: fn(&mut Sm),
    handler: fn(&mut Sm, &Sm::Event) -> Action<Sm>,
    exit: fn(&mut Sm),
}

impl<Sm: StateMachineDef> PartialEq for StateDesc<Sm> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
