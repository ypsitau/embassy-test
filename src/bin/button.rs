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
    let gpio_button = gpio::Input::new(p.PIN_15, gpio::Pull::Up);
    let mut state_prev = gpio_button.is_low();
    loop {
        let state = gpio_button.is_low();
        if state == state_prev {
            // no change
        } else if state {
            info!("button pressed!");
            gpio_led.set_high();
        } else {
            info!("button released!");
            gpio_led.set_low();
        }
        state_prev = state;
        Timer::after_millis(100).await;
    }
}
