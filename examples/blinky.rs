#![allow(clippy::empty_loop)]
#![no_main]
#![no_std]

use panic_halt as _;

use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;
use stm32f4xx_hal as hal;

use hal::{
    gpio::{self, Edge, Input, Output, PushPull},
    pac::{self, TIM2, interrupt},
    prelude::*,
    rcc::Config,
    timer::{CounterMs, Event},
};

use rsm::{
    Action, Actor, EventProducer, StateImpl, StateMachineSpec, Mailbox, MpmcBoundedQueue, Parent, StateRef, State, Step,
    Top,
};

#[derive(Debug)]
enum BlinkEvent {
    UpdateTimeout,
    Timeout,
}

struct Blinky {
    led: gpio::PA5<Output<PushPull>>,
    max_update_rate: u32,
    update_rate_adjust: u32,
    update_rate: u32,
}

impl StateMachineSpec for Blinky {
    type Event = BlinkEvent;

    fn initial(&mut self) -> State<Self> {
        LedOn::state()
    }
}

struct BlinkyTop;

impl StateImpl<BlinkyTop> for Blinky {
    type Parent = Top;

    fn handler(&mut self, event: &BlinkEvent) -> Action<Self> {
        match event {
            BlinkEvent::UpdateTimeout => {
                self.update_rate -= self.update_rate_adjust;

                if self.update_rate < self.update_rate_adjust {
                    self.update_rate = self.max_update_rate;
                }

                cortex_m::interrupt::free(|cs| {
                    if let Some(timer) = TIMER.borrow(cs).borrow_mut().as_mut() {
                        timer.start(self.update_rate.millis()).unwrap();
                    }
                });

                Action::<Self>::Handled
            }
            _ => Action::<Self>::Unhandled,
        }
    }
}

struct LedOn;

impl StateImpl<LedOn> for Blinky {
    type Parent = Parent<BlinkyTop>;

    fn entry(&mut self) {
        self.led.set_high();
    }

    fn handler(&mut self, event: &BlinkEvent) -> Action<Self> {
        match event {
            BlinkEvent::Timeout => Action::<Self>::Transition(LedOff::state()),
            _ => Action::<Self>::Unhandled,
        }
    }
}

struct LedOff;

impl StateImpl<LedOff> for Blinky {
    type Parent = Parent<BlinkyTop>;

    fn entry(&mut self) {
        self.led.set_low();
    }

    fn handler(&mut self, event: &BlinkEvent) -> Action<Self> {
        match event {
            BlinkEvent::Timeout => Action::<Self>::Transition(LedOn::state()),
            _ => Action::<Self>::Unhandled,
        }
    }
}

type BlinkEventQueue = MpmcBoundedQueue<BlinkEvent, 32>;

static MAILBOX: Mailbox<BlinkEvent, BlinkEventQueue> = Mailbox::new(BlinkEventQueue::new());
static PRODUCER: Mutex<RefCell<Option<EventProducer<BlinkEvent, BlinkEventQueue>>>> =
    Mutex::new(RefCell::new(None));
static TIMER: Mutex<RefCell<Option<CounterMs<TIM2>>>> = Mutex::new(RefCell::new(None));
static BUTTON: Mutex<RefCell<Option<gpio::PC13<Input>>>> = Mutex::new(RefCell::new(None));

#[interrupt]
fn TIM2() {
    cortex_m::interrupt::free(|cs| {
        if let Some(producer) = PRODUCER.borrow(cs).borrow().as_ref() {
            let _ = producer.enqueue(BlinkEvent::Timeout);
        }

        if let Some(timer) = TIMER.borrow(cs).borrow_mut().as_mut() {
            timer.clear_all_flags();
        }
    });
}

#[interrupt]
fn EXTI15_10() {
    cortex_m::interrupt::free(|cs| {
        if let Some(producer) = PRODUCER.borrow(cs).borrow().as_ref() {
            let _ = producer.enqueue(BlinkEvent::UpdateTimeout);
        }

        if let Some(button) = BUTTON.borrow(cs).borrow_mut().as_mut() {
            button.clear_interrupt_pending_bit();
        }
    });
}

#[entry]
fn main() -> ! {
    if let Some(mut dp) = pac::Peripherals::take() {
        let mut rcc = dp.RCC.freeze(Config::hsi().sysclk(48.MHz()));

        let gpioa = dp.GPIOA.split(&mut rcc);
        let led = gpioa.pa5.into_push_pull_output();

        let gpioc = dp.GPIOC.split(&mut rcc);
        let mut button = gpioc.pc13;

        let mut syscfg = dp.SYSCFG.constrain(&mut rcc);
        button.make_interrupt_source(&mut syscfg);
        button.trigger_on_edge(&mut dp.EXTI, Edge::Falling);
        button.enable_interrupt(&mut dp.EXTI);

        let mut timer = dp.TIM2.counter_ms(&mut rcc);
        let init_update_rate = 2000;

        timer.start(init_update_rate.millis()).unwrap();
        timer.listen(Event::Update);

        unsafe {
            cortex_m::peripheral::NVIC::unmask(interrupt::TIM2);
            cortex_m::peripheral::NVIC::unmask(button.interrupt());
        }

        let context = Blinky {
            led,
            max_update_rate: init_update_rate,
            update_rate_adjust: 500,
            update_rate: init_update_rate,
        };
        let (producer, consumer) = MAILBOX.split().unwrap();

        cortex_m::interrupt::free(|cs| {
            PRODUCER.borrow(cs).replace(Some(producer));
            TIMER.borrow(cs).replace(Some(timer));
            BUTTON.borrow(cs).replace(Some(button));
        });

        let mut actor = Actor::<Blinky, BlinkEventQueue>::new(context, consumer);

        loop {
            while actor.step() {}
            cortex_m::asm::wfi();
        }
    }

    loop {}
}
