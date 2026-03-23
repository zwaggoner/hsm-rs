use core::marker::PhantomData;

pub type State<Sm> = &'static StateDesc<Sm>;

pub trait StateMachineSpec: Sized {
    type Event: 'static;

    fn initial(&mut self) -> State<Self>;
}

pub enum Action<Sm: StateMachineSpec + 'static> {
    Unhandled,
    Handled,
    Transition(State<Sm>),
}

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

pub trait ParentState<Sm: StateMachineSpec + 'static>: _private::Sealed {
    const OPT_STATE: Option<State<Sm>>;
}

pub struct Parent<S>(PhantomData<S>);
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

