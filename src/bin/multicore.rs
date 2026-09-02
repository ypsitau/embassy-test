#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Executor;
use embassy_rp as rp;
use embassy_sync as sync;
//use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
//use embassy_sync::channel::Channel;
use embassy_time::Timer;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

enum LedState {
    On,
    Off,
}

static CHANNEL: sync::channel::Channel<sync::blocking_mutex::raw::CriticalSectionRawMutex, LedState, 1> = sync::channel::Channel::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = rp::init(Default::default());
    let gpio_led = rp::gpio::Output::new(p.PIN_25, rp::gpio::Level::Low);
    let stack_core1 = {
        static STATIC_CELL: StaticCell<rp::multicore::Stack<4096>> = StaticCell::new();
        STATIC_CELL.init(rp::multicore::Stack::new())
    };
    rp::multicore::spawn_core1(p.CORE1, stack_core1,
        move || {
            let executor_core1 = {
                static STATIC_CELL: StaticCell<Executor> = StaticCell::new();
                STATIC_CELL.init(Executor::new())
            };
            executor_core1.run(|spawner| spawner.spawn(unwrap!(task_core1(gpio_led))));
        },
    );
    let executor_core0 = {
        static STATIC_CELL: StaticCell<Executor> = StaticCell::new();
        STATIC_CELL.init(Executor::new())
    };
    executor_core0.run(|spawner| spawner.spawn(unwrap!(task_core0())));
}

#[embassy_executor::task]
async fn task_core0() {
    info!("Hello from core 0");
    loop {
        CHANNEL.send(LedState::On).await;
        Timer::after_millis(100).await;
        CHANNEL.send(LedState::Off).await;
        Timer::after_millis(400).await;
    }
}

#[embassy_executor::task]
async fn task_core1(mut gpio_led: rp::gpio::Output<'static>) {
    info!("Hello from core 1");
    loop {
        match CHANNEL.receive().await {
            LedState::On => gpio_led.set_high(),
            LedState::Off => gpio_led.set_low(),
        }
    }
}
