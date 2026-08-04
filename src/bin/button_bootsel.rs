//! This example reads the onboard bootselect button and reports the value on a serial connection.

#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::bootsel::is_bootsel_pressed;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut p = embassy_rp::init(Default::default());
    let mut previous = false;
    loop {
        Timer::after_micros(10).await;
        let pressed = is_bootsel_pressed(p.BOOTSEL.reborrow());
        if pressed != previous {
            info!("bootsel is now {}", pressed);
        }
        previous = pressed;
    }
}
