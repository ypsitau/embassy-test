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
            static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| spawner.spawn(unwrap!(core1_task(gpio_led))));
        },
    );

    static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| spawner.spawn(unwrap!(core0_task())));
}

#[embassy_executor::task]
async fn core0_task() {
    info!("Hello from core 0");
    loop {
        CHANNEL.send(LedState::On).await;
        Timer::after_millis(100).await;
        CHANNEL.send(LedState::Off).await;
        Timer::after_millis(400).await;
    }
}

#[embassy_executor::task]
async fn core1_task(mut gpio_led: rp::gpio::Output<'static>) {
    info!("Hello from core 1");
    loop {
        match CHANNEL.receive().await {
            LedState::On => gpio_led.set_high(),
            LedState::Off => gpio_led.set_low(),
        }
    }
}
