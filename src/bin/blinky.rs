#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::gpio;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut gpio_led = gpio::Output::new(p.PIN_25, gpio::Level::Low);
    loop {
        info!("led on!");
        gpio_led.set_high();
        Timer::after_secs(1).await;
        info!("led off!");
        gpio_led.set_low();
        Timer::after_secs(1).await;
    }
}
