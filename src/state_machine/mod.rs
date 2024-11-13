mod core;
mod runtime;

use self::core::DEFAULT_MAX_NEST_DEPTH;
pub use self::core::{Action, State, StateDef, StateMachineDef, StateRef, Super, Top};

pub use self::runtime::StateMachine;
pub(crate) use self::runtime::{Init, Run};
