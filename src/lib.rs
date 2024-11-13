#![no_std]
#![warn(missing_docs)]

//! hsm-rs is a hierarchical state machine (hsm) implementation in rust (rs). The development of this state machine framework was inspired by the [QuantumLeaps](https://www.state-machine.com/) state machine framework developed by Miro Samek, and is arguably a partial port of the framework to Rust. The goals of this framework was to provide a `no_std`/no allocation state machine framework in rust, balancing the goals of being idiomatic, suitable for embedded development, and equivalence to the quantum leaps state machine framework, especially regarding transition semantics.
//!
//! The API for hsm-rs is similar to the older [kaori-hsm](https://docs.rs/kaori-hsm/latest/kaori_hsm). This similarity is accidental, as the API was largely shaped prior to the discovery of kaori-hsm by the author. Some of the key differences between the two frameworks are:
//! - Use of structs (manual vtables) for dispatch in hsm-rs vs hidden trait function for dispatch in kaori-hsm
//! - Encoding of Parent relationships in the state hierarchy in hsm-rs to get compile-time failures for circular parent dependencies
//! - Optional and additional actor framework extending the state machine framework
//! - All state machine specifications in idiomatic rust, no macros
//!
//! The quantum leaps website provides more resources for understanding hierarchical state machines than I could possibly cover here so I will defer to the content on his website for explaining the concept.
//!
//! This project started primarily to build the author's familiarity with Rust, so feedback/suggestions are certainly welcome.
//!
//! For background on hierarchical state machines generally, the links above provide a far superior and more in-depth background than I could provide here, so the documentation will focus on the particularities of this framework.
//!
//! To get started, please look at the examples in the following order:
//! 1. [`StateMachineDef`] for defining your active object/state machine context.
//! 2. [`StateDef`] for defining states.
//! 3. [`StateMachine`] if running the state machine without an actor, otherwise.
//! 4. [`actor`] if using the actor execution framework
//!
//! For adapting different queue implementations to the actor framework,
//! please view [`actor::queue`]

/// The `actor` module provides wrappers around state machine objects to provide an active-object
/// (actor) framework. The actors include event queue integration, and simple run-to-completion
/// `step` semantics.  
///
/// To use an `actor`, once you have your state machine fully defined, there's a couple of different
/// ways to instantiate an actor. In the case of a normal runtime you may instantiate your queue and
/// actor directly in main:
/// ```
/// # use hsm_rs::*;
/// # use hsm_rs::actor::*;
/// # use hsm_rs::actor::queue::*;
/// #
/// # #[derive(Debug)]
/// # enum MyEvent {
/// #    Event1,
/// #    Event2,
/// # }
/// #
/// # struct MyActor;
/// # impl MyActor {
/// #   fn new() -> Self {
/// #       Self {}
/// #   }
/// # }
/// #
/// # impl StateMachineDef for MyActor {
/// #    type Event = MyEvent;
/// #
/// #    fn initial(&mut self) -> State<Self> {
/// #        State1::state()
/// #    }
/// # }
/// #
/// # struct State1;
/// #
/// # impl StateDef<State1> for MyActor {
/// #   type Parent = Top;
/// # }
/// fn main () {
///    // Instantiate a queue for MyEvent with a capacity of 32 elements
///    // Note that queue's and actors do not need to be mutable
///    let queue = MpmcBoundedQueue::<MyEvent, 32>::new();
///    
///    // Actor takes the queue
///    let actor = Actor::new(queue);
///
///    // Actor context does need to be mutable since it is taken
///    let my_actor = MyActor::new();
///
///    // Runtime actor does need to be mutable, actor can only be bound once
///    let mut actor_rt = actor.bind(my_actor);
///
///    // Producer handle can only be taken once but can be taken at any time
///    // Note MultiProducer capable handles can be cloned, but the limit on take_producer still
///    // applies.
///    let Some(mut producer) = actor.take_producer() else { panic!(); };
///
///    // Actor can be queried for initialization.
///    // But, will always initialize on the first call to step if it is not
///    println!("actor_rt.initialized(): {}", actor_rt.initialized());
///
///    // The step status can be examined
///    match actor_rt.step() {
///        StepStatus::Initialized { pending } => println!("Actor initalized, with pending set to: {}", pending),
///        StepStatus::Ran { pending } => println!("Actor ran (dispatched an event) with pending set to: {}", pending),
///        StepStatus::Idle => println!("Actor was idle with no events dispatched"),
///    }
///
///    // Events can be queued using the producer handle, failures can be handled by the user as
///    // needed.
///    match producer.enqueue(MyEvent::Event1) {
///        Ok(_) => println!("Successful enqueue"),
///        Err(obj) => println!("Enqueue failed for {:?}", obj),
///    }
///
///    // `MultiProducer` queues can also enqueue directly on the actor (original, not the runtime)
///    actor.enqueue(MyEvent::Event2).unwrap();
/// }
///
/// ```
/// The different phases of initialization are explicitly laid out separately for clarity, but can
/// be combined. This kind of interface design for the actors may look like overkill in the above example, but is very useful
/// in the embedded case, as demonstrated in the [blinky](https://github.com/zwaggoner/hsm-rs/blob/master/examples/blinky.rs) example,
/// which also demonstrates use of the superloop scheduler. See [`actor::runtime`] for an overview of the available
/// runtimes.
pub mod actor;
mod state_machine;
mod util;

pub use crate::state_machine::{
    Action, State, StateDef, StateMachine, StateMachineDef, StateRef, Super, Top,
};
