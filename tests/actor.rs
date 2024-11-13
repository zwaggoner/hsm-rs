extern crate hsm_rs;
extern crate std;

use hsm_rs::actor::{Actor, ActorRuntime, StepStatus, queue::MpmcBoundedQueue};
use hsm_rs::{State, StateDef, StateMachineDef, StateRef, Top};

#[derive(Debug)]
enum TestEvent {
    Event1,
}

struct TestActor;
struct State1;

impl StateMachineDef for TestActor {
    type Event = TestEvent;

    fn initial(&mut self) -> State<Self> {
        State1::state()
    }
}

impl StateDef<State1> for TestActor {
    type Parent = Top;
}

type EventQueue = MpmcBoundedQueue<TestEvent, 2>;

#[test]
fn test_take_producer() {
    let actor = Actor::<TestActor, EventQueue>::new(EventQueue::default());

    assert!(actor.take_producer().is_some());
}

#[test]
fn test_take_producer_once() {
    let actor = Actor::<TestActor, EventQueue>::new(EventQueue::default());
    let _producer = actor.take_producer();

    assert!(actor.take_producer().is_none());
}

#[test]
fn test_bind() {
    let actor = Actor::<TestActor, EventQueue>::new(EventQueue::default());

    let _actor_rt = actor.bind(TestActor {});
}

#[test]
#[should_panic]
fn test_bind_once() {
    let actor = Actor::<TestActor, EventQueue>::new(EventQueue::default());

    let _actor_rt = actor.bind(TestActor {});
    let _actor_rt2 = actor.bind(TestActor {});
}

#[test]
fn test_uninitialized() {
    let actor = Actor::<TestActor, EventQueue>::new(EventQueue::default());

    let actor_rt = actor.bind(TestActor {});

    assert!(!actor_rt.initialized());
}

#[test]
fn test_initialize_on_first_step_no_pending() {
    let actor = Actor::<TestActor, EventQueue>::new(EventQueue::default());

    let mut actor_rt = actor.bind(TestActor {});

    assert!(matches!(
        actor_rt.step(),
        StepStatus::Initialized { pending: false }
    ));
}

#[test]
fn test_direct_enqueue_on_multi_producer() {
    let actor = Actor::<TestActor, EventQueue>::new(EventQueue::default());
    assert!(actor.enqueue(TestEvent::Event1).is_ok());
}

#[test]
fn test_initialize_on_first_step_pending() {
    let actor = Actor::<TestActor, EventQueue>::new(EventQueue::default());
    actor
        .enqueue(TestEvent::Event1)
        .expect("Queue unexpectedly full");

    let mut actor_rt = actor.bind(TestActor {});

    assert!(matches!(
        actor_rt.step(),
        StepStatus::Initialized { pending: true }
    ));
}

#[test]
fn test_ran_no_pending() {
    let actor = Actor::<TestActor, EventQueue>::new(EventQueue::default());

    let mut actor_rt = actor.bind(TestActor {});
    let _ = actor_rt.step();
    actor
        .enqueue(TestEvent::Event1)
        .expect("Queue unexpectedly full");
    assert!(matches!(
        actor_rt.step(),
        StepStatus::Ran { pending: false }
    ));
}

#[test]
fn test_ran_pending() {
    let actor = Actor::<TestActor, EventQueue>::new(EventQueue::default());

    let mut actor_rt = actor.bind(TestActor {});
    let _ = actor_rt.step();
    actor
        .enqueue(TestEvent::Event1)
        .expect("Queue unexpectedly full");
    actor
        .enqueue(TestEvent::Event1)
        .expect("Queue unexpectedly full");
    assert!(matches!(actor_rt.step(), StepStatus::Ran { pending: true }));
}

#[test]
fn test_idle() {
    let actor = Actor::<TestActor, EventQueue>::new(EventQueue::default());

    let mut actor_rt = actor.bind(TestActor {});
    let _ = actor_rt.step();
    assert!(matches!(actor_rt.step(), StepStatus::Idle));
}

#[test]
fn test_idle_is_idle() {
    assert!(StepStatus::Idle.is_idle());
}

#[test]
fn test_initialized_and_pending_is_not_idle() {
    assert!(!StepStatus::Initialized { pending: true }.is_idle());
}

#[test]
fn test_initialized_and_not_pending_is_not_idle() {
    assert!(!StepStatus::Initialized { pending: false }.is_idle());
}

#[test]
fn test_ran_and_pending_is_not_idle() {
    assert!(!StepStatus::Ran { pending: true }.is_idle());
}

#[test]
fn test_ran_and_not_pending_is_not_idle() {
    assert!(!StepStatus::Ran { pending: false }.is_idle());
}

#[test]
fn test_idle_is_not_pending() {
    assert!(!StepStatus::Idle.is_pending());
}

#[test]
fn test_initialized_and_pending_is_pending() {
    assert!(StepStatus::Initialized { pending: true }.is_pending());
}

#[test]
fn test_initialized_and_not_pending_is_not_pending() {
    assert!(!StepStatus::Initialized { pending: false }.is_pending());
}

#[test]
fn test_ran_and_pending_is_pending() {
    assert!(StepStatus::Ran { pending: true }.is_pending());
}

#[test]
fn test_ran_and_not_pending_is_not_pending() {
    assert!(!StepStatus::Ran { pending: false }.is_pending());
}

#[test]
fn test_idle_not_did_work() {
    assert!(!StepStatus::Idle.did_work());
}

#[test]
fn test_initialized_and_pending_did_work() {
    assert!(StepStatus::Initialized { pending: true }.did_work());
}

#[test]
fn test_initialized_and_not_pending_did_work() {
    assert!(StepStatus::Initialized { pending: false }.did_work());
}

#[test]
fn test_ran_and_pending_did_work() {
    assert!(StepStatus::Ran { pending: true }.did_work());
}

#[test]
fn test_ran_and_not_pending_did_work() {
    assert!(StepStatus::Ran { pending: false }.did_work());
}
