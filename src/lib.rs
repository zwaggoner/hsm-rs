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

pub mod actor;
mod state_machine;
mod util;

pub use crate::state_machine::{
    Action, ParentState, State, StateDef, StateMachine, StateMachineDef, StateRef, Super, Top,
};
