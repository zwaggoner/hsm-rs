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
    pac::{self, TIM2, TIM3, interrupt},
    prelude::*,
    rcc::Config,
    timer::{CounterMs, Event},
};

use rsm::{
    Action, Actor, EventProducer, Mailbox, MpmcBoundedQueue, Parent, State, StateImpl,
    StateMachineSpec, StateRef, Step, Top,
};

#[derive(Debug)]
enum BlinkEvent {
    ButtonPress,
    DebounceTimeout,
    Timeout,
}

type LedType = gpio::PA5<Output<PushPull>>;

struct Blinky {
    led: LedType,
    divisor: u32,
}

impl Blinky {
    const MAX_UPDATE_RATE: u32 = 2000;
    const MAX_DIVISOR: u32 = 4;

    fn new(led: LedType) -> Self {
        Self { led, divisor: 1 }
    }
}

impl StateMachineSpec for Blinky {
    type Event = BlinkEvent;

    fn initial(&mut self) -> State<Self> {
        cortex_m::interrupt::free(|cs| {
            if let Some(shared) = SHARED.borrow(cs).borrow_mut().as_mut() {
                shared
                    .blink_timer
                    .start(Self::MAX_UPDATE_RATE.millis())
                    .unwrap();
                shared.blink_timer.listen(Event::Update);
            }
        });

        LedOn::state()
    }
}

struct BlinkyTop;

impl StateImpl<BlinkyTop> for Blinky {
    type Parent = Top;

    fn handler(&mut self, event: &BlinkEvent) -> Action<Self> {
        match event {
            BlinkEvent::ButtonPress => {
                cortex_m::interrupt::free(|cs| {
                    if let Some(shared) = SHARED.borrow(cs).borrow_mut().as_mut() {
                        shared.debounce_timer.start(10.millis()).unwrap();
                        shared.debounce_timer.listen(Event::Update);
                    }
                });

                Action::Handled
            }
            BlinkEvent::DebounceTimeout => {
                let mut button_state = false;

                cortex_m::interrupt::free(|cs| {
                    if let Some(shared) = SHARED.borrow(cs).borrow_mut().as_mut() {
                        button_state = shared.button.is_high();
                    }
                });

                if button_state {
                    self.divisor += 1;

                    if self.divisor > Self::MAX_DIVISOR {
                        self.divisor = 1;
                    }

                    let update_rate = Self::MAX_UPDATE_RATE / self.divisor;

                    cortex_m::interrupt::free(|cs| {
                        if let Some(shared) = SHARED.borrow(cs).borrow_mut().as_mut() {
                            shared.blink_timer.start(update_rate.millis()).unwrap();
                        }
                    });
                }

                cortex_m::interrupt::free(|cs| {
                    if let Some(shared) = SHARED.borrow(cs).borrow_mut().as_mut() {
                        unsafe {
                            shared.button.clear_interrupt_pending_bit();
                            cortex_m::peripheral::NVIC::unmask(shared.button.interrupt());
                        }
                    }
                });

                Action::Handled
            }
            _ => Action::Unhandled,
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
            BlinkEvent::Timeout => Action::Transition(LedOff::state()),
            _ => Action::Unhandled,
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
            BlinkEvent::Timeout => Action::Transition(LedOn::state()),
            _ => Action::Unhandled,
        }
    }
}

type BlinkEventQueue = MpmcBoundedQueue<BlinkEvent, 32>;

static MAILBOX: Mailbox<BlinkEvent, BlinkEventQueue> = Mailbox::new(BlinkEventQueue::new());

struct Shared {
    producer: EventProducer<'static, BlinkEvent, BlinkEventQueue>,
    blink_timer: CounterMs<TIM2>,
    debounce_timer: CounterMs<TIM3>,
    button: gpio::PC13<Input>,
}

static SHARED: Mutex<RefCell<Option<Shared>>> = Mutex::new(RefCell::new(None));

#[interrupt]
fn TIM2() {
    cortex_m::interrupt::free(|cs| {
        if let Some(shared) = SHARED.borrow(cs).borrow_mut().as_mut() {
            let _ = shared.producer.enqueue(BlinkEvent::Timeout);
            shared.blink_timer.clear_all_flags();
        }
    });
}

#[interrupt]
fn TIM3() {
    cortex_m::interrupt::free(|cs| {
        if let Some(shared) = SHARED.borrow(cs).borrow_mut().as_mut() {
            let _ = shared.producer.enqueue(BlinkEvent::DebounceTimeout);
            shared.debounce_timer.clear_all_flags();
            let _ = shared.debounce_timer.cancel();
        }
    });
}

#[interrupt]
fn EXTI15_10() {
    cortex_m::interrupt::free(|cs| {
        if let Some(shared) = SHARED.borrow(cs).borrow_mut().as_mut() {
            let _ = shared.producer.enqueue(BlinkEvent::ButtonPress);
            cortex_m::peripheral::NVIC::mask(shared.button.interrupt());
            shared.button.clear_interrupt_pending_bit();
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
        button.trigger_on_edge(&mut dp.EXTI, Edge::Rising);
        button.enable_interrupt(&mut dp.EXTI);

        let blink_timer = dp.TIM2.counter_ms(&mut rcc);
        let debounce_timer = dp.TIM3.counter_ms(&mut rcc);

        unsafe {
            cortex_m::peripheral::NVIC::unmask(interrupt::TIM2);
            cortex_m::peripheral::NVIC::unmask(interrupt::TIM3);
            cortex_m::peripheral::NVIC::unmask(button.interrupt());
        }

        let context = Blinky::new(led);

        let (producer, consumer) = MAILBOX.split().unwrap();

        cortex_m::interrupt::free(|cs| {
            SHARED.borrow(cs).replace(Some(Shared {
                producer,
                blink_timer,
                debounce_timer,
                button,
            }));
        });

        let mut actor = Actor::<Blinky, BlinkEventQueue>::new(context, consumer);

        loop {
            while actor.step() {}
            cortex_m::asm::wfi();
        }
    }

    loop {}
}
