// see https://github.com/adamgreig/assign-resources for more information

#![no_std]
#![no_main]

use assign_resources::assign_resources;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_rp::{Peri, peripherals};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = rp::init(Default::default());
    let r = split_resources!(p);
    spawner.spawn(task1(spawner, r.resources_task1).unwrap());
    spawner.spawn(task2(spawner, r.resources_task2).unwrap());
}

assign_resources! {
    resources_task1: ResourcesTask1{
        pin_20: PIN_20,
        pin_21: PIN_21,
    }
    resources_task2: ResourcesTask2{
        pin_10: PIN_10,
        pin_11: PIN_11,
    }
}

#[embassy_executor::task]
async fn task1(_spawner: Spawner, r: ResourcesTask1) {
    let mut gpio_20 = rp::gpio::Output::new(r.pin_20, rp::gpio::Level::Low);
    let mut gpio_21 = rp::gpio::Output::new(r.pin_21, rp::gpio::Level::High);
    loop {
        info!("toggling leds");
        gpio_20.toggle();
        gpio_21.toggle();
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn task2(_spawner: Spawner, r: ResourcesTask2) {
    let mut gpio_10 = rp::gpio::Output::new(r.pin_10, rp::gpio::Level::Low);
    let mut gpio_11 = rp::gpio::Output::new(r.pin_11, rp::gpio::Level::High);
    loop {
        info!("toggling leds");
        gpio_10.toggle();
        gpio_11.toggle();
        Timer::after_secs(1).await;
    }
}
