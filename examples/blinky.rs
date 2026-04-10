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
    Action, State, StateDef, StateMachineDef, StateRef, Super, Top,
    actor::{
        Actor,
        queue::MpmcBoundedQueue,
        runtime::Superloop,
    },
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

    fn configure_blink_timer(&mut self) {
        let update_rate = Self::MAX_UPDATE_RATE / self.divisor;

        cortex_m::interrupt::free(|cs| {
            if let Some(shared) = SHARED.borrow(cs).borrow_mut().as_mut() {
                shared.blink_timer.start(update_rate.millis()).unwrap();
            }
        });
    }

    fn get_button_state(&mut self) -> bool {
        let mut button_state = false;

        cortex_m::interrupt::free(|cs| {
            if let Some(shared) = SHARED.borrow(cs).borrow_mut().as_mut() {
                button_state = shared.button.is_high();
            }
        });

        button_state
    }

    fn enable_button_event(&mut self) {
        cortex_m::interrupt::free(|cs| {
            if let Some(shared) = SHARED.borrow(cs).borrow_mut().as_mut() {
                unsafe {
                    shared.button.clear_interrupt_pending_bit();
                    cortex_m::peripheral::NVIC::unmask(shared.button.interrupt());
                }
            }
        });
    }

    fn start_debounce_timer(&mut self) {
        cortex_m::interrupt::free(|cs| {
            if let Some(shared) = SHARED.borrow(cs).borrow_mut().as_mut() {
                shared.debounce_timer.start(20.millis()).unwrap();
                shared.debounce_timer.listen(Event::Update);
            }
        });
    }
}

impl StateMachineDef for Blinky {
    type Event = BlinkEvent;

    fn initial(&mut self) -> State<Self> {
        self.configure_blink_timer();
        LedOn::state()
    }
}

struct BlinkyTop;

impl StateDef<BlinkyTop> for Blinky {
    type Parent = Top;

    fn handler(&mut self, event: &BlinkEvent) -> Action<Self> {
        match event {
            BlinkEvent::ButtonPress => {
                self.start_debounce_timer();

                Action::Handled
            }
            BlinkEvent::DebounceTimeout => {
                if self.get_button_state() {
                    self.divisor += 1;

                    if self.divisor > Self::MAX_DIVISOR {
                        self.divisor = 1;
                    }

                    self.configure_blink_timer();
                }

                self.enable_button_event();
                Action::Handled
            }
            _ => Action::Unhandled,
        }
    }
}

struct LedOn;

impl StateDef<LedOn> for Blinky {
    type Parent = Super<BlinkyTop>;

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

impl StateDef<LedOff> for Blinky {
    type Parent = Super<BlinkyTop>;

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

static ACTOR: Actor<Blinky, BlinkEventQueue> = Actor::new(BlinkEventQueue::new());

struct Shared {
    blink_timer: CounterMs<TIM2>,
    debounce_timer: CounterMs<TIM3>,
    button: gpio::PC13<Input>,
}

static SHARED: Mutex<RefCell<Option<Shared>>> = Mutex::new(RefCell::new(None));

#[interrupt]
fn TIM2() {
    let _ = ACTOR.enqueue(BlinkEvent::Timeout);

    cortex_m::interrupt::free(|cs| {
        if let Some(shared) = SHARED.borrow(cs).borrow_mut().as_mut() {
            shared.blink_timer.clear_all_flags();
        }
    });
}

#[interrupt]
fn TIM3() {
    let _ = ACTOR.enqueue(BlinkEvent::DebounceTimeout);

    cortex_m::interrupt::free(|cs| {
        if let Some(shared) = SHARED.borrow(cs).borrow_mut().as_mut() {
            let _ = shared.debounce_timer.cancel();
            shared.debounce_timer.clear_all_flags();
        }
    });
}

#[interrupt]
fn EXTI15_10() {
    cortex_m::interrupt::free(|cs| {
        if let Some(shared) = SHARED.borrow(cs).borrow_mut().as_mut() {
            cortex_m::peripheral::NVIC::mask(shared.button.interrupt());
            shared.button.clear_interrupt_pending_bit();
        }
    });

    let _ = ACTOR.enqueue(BlinkEvent::ButtonPress);
}

#[entry]
fn main() -> ! {
    if let Some(mut dp) = pac::Peripherals::take() {
        // Always configure your clocks first
        let mut rcc = dp.RCC.freeze(Config::hsi().sysclk(48.MHz()));

        // Configure LED GPIO
        let gpioa = dp.GPIOA.split(&mut rcc);
        let led = gpioa.pa5.into_push_pull_output();

        // Configure Button GPIO
        let gpioc = dp.GPIOC.split(&mut rcc);
        let mut button = gpioc.pc13;

        // Get syscfg HAL
        let mut syscfg = dp.SYSCFG.constrain(&mut rcc);

        // Configure button inputs/events
        button.make_interrupt_source(&mut syscfg);
        button.trigger_on_edge(&mut dp.EXTI, Edge::Rising);
        button.enable_interrupt(&mut dp.EXTI);

        // Setup blink timer
        let mut blink_timer = dp.TIM2.counter_ms(&mut rcc);
        blink_timer.listen(Event::Update);

        // Setup debounce timer
        let debounce_timer = dp.TIM3.counter_ms(&mut rcc);

        // Construct the Blinky context object
        let context = Blinky::new(led);

        // Unmask all of the interrupts we are going to use
        unsafe {
            cortex_m::peripheral::NVIC::unmask(interrupt::TIM2);
            cortex_m::peripheral::NVIC::unmask(interrupt::TIM3);
            cortex_m::peripheral::NVIC::unmask(button.interrupt());
        }

        // Configure the shared object with everything needed in the ISR context
        cortex_m::interrupt::free(|cs| {
            SHARED.borrow(cs).replace(Some(Shared {
                blink_timer,
                debounce_timer,
                button,
            }));
        });

        // Configure the blinky actor with the context object and consumer
        Superloop::new(
            [&mut ACTOR.bind(context)],
            Some(|| {
                cortex_m::asm::wfi();
            }),
        )
        .run();
    }

    loop {}
}
