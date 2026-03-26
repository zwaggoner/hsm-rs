#![no_std]

//! A state machine/actor framework inspired by the [QuantumLeaps](https://www.state-machine.com/)
//! framework, ported to Rust. The goals of this framework were to provide a `no_std`/no allocation
//! version of a state machine framework in rust, while being as idiomatic as possible (based upon
//! the understanding of a for a first-time rust developer. The goal of this framework was primarily to be educational for the
//! author, there are other rust state machine/actor frameworks that may be more suitable for your
//! purpose.
//! 
//! To get started with this framework, please look at the examples for [`StateMachineDef`] followed by
//! [`StateDef`]. 

mod actor;
mod event_queue;
mod fixed_vec;
mod mpmc_bounded_queue;
mod runtime;
mod state_machine;
mod state_machine_runtime;

pub use actor::{Actor, ActorRuntime};
pub use event_queue::{EventProducer, EventQueue, QueueAdapter};
pub use mpmc_bounded_queue::MpmcBoundedQueue;
pub use runtime::{Cooperative, Runtime, Superloop};
pub use state_machine::{Action, State, StateDef, StateMachineDef, StateRef, Super, Top};
pub use state_machine_runtime::{Init, Run, StateMachine};
