//! This example demonstrates how to assign resources to multiple tasks by splitting up the peripherals.
//! It is not about sharing the same resources between tasks, see sharing.rs for that or head to https://embassy.dev/book/#_sharing_peripherals_between_tasks)
//! Of course splitting up resources and sharing resources can be combined, yet this example is only about splitting up resources.
//!
//! There are basically two ways we demonstrate here:
//! 1) Assigning resources to a task by passing parts of the peripherals
//! 2) Assigning resources to a task by passing a struct with the split up peripherals, using the assign-resources macro
//!
//! using four LEDs on Pins 10, 11, 20 and 21

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
    // initialize the peripherals
    let p = rp::init(Default::default());

    // 1) Assigning a resource to a task by passing parts of the peripherals.
    spawner.spawn(task_manually_assigned(spawner, p.PIN_20, p.PIN_21).unwrap());

    // 2) Using the assign-resources macro to assign resources to a task.
    // we perform the split, see further below for the definition of the resources struct
    let r = split_resources!(p);
    // and then we can use them
    spawner.spawn(task_macro_assigned(spawner, r.led_resources).unwrap());
}

// 1) Assigning a resource to a task by passing parts of the peripherals.
#[embassy_executor::task]
async fn task_manually_assigned(_spawner: Spawner,
    pin_20: rp::Peri<'static, rp::peripherals::PIN_20>,
    pin_21: rp::Peri<'static, rp::peripherals::PIN_21>,
) {
    let mut gpio_20 = rp::gpio::Output::new(pin_20, rp::gpio::Level::Low);
    let mut gpio_21 = rp::gpio::Output::new(pin_21, rp::gpio::Level::High);

    loop {
        info!("toggling leds");
        gpio_20.toggle();
        gpio_21.toggle();
        Timer::after_secs(1).await;
    }
}

// 2) Using the assign-resources macro to assign resources to a task.
// first we define the resources we want to assign to the task using the assign_resources! macro
// basically this will split up the peripherals struct into smaller structs, that we define here
// naming is up to you, make sure your future self understands what you did here
assign_resources! {
    led_resources: LedResources{
        pin_10: PIN_10,
        pin_11: PIN_11,
    }
    // add more resources to more structs if needed, for example defining one struct for each task
}
// this could be done in another file and imported here, but for the sake of simplicity we do it here
// see https://github.com/adamgreig/assign-resources for more information

// 2) Using the split resources in a task
#[embassy_executor::task]
async fn task_macro_assigned(_spawner: Spawner, r: LedResources) {
    let mut gpio_10 = rp::gpio::Output::new(r.pin_10, rp::gpio::Level::Low);
    let mut gpio_11 = rp::gpio::Output::new(r.pin_11, rp::gpio::Level::High);

    loop {
        info!("toggling leds");
        gpio_10.toggle();
        gpio_11.toggle();
        Timer::after_secs(1).await;
    }
}
