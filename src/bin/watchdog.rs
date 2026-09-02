#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    info!("Hello world!");
    let mut watchdog = rp::watchdog::Watchdog::new(p.WATCHDOG);
    let mut gpio_led = rp::gpio::Output::new(p.PIN_25, rp::gpio::Level::Low);
    gpio_led.set_high();
    Timer::after_secs(2).await;
    // Set to watchdog to reset if it's not fed within 1.05 seconds, and start it
    watchdog.start(Duration::from_millis(5_050));
    info!("Started the watchdog timer");
    Timer::after_millis(4_000).await;
    // Blink once a second for 5 seconds, feed the watchdog timer once a second to avoid a reset
    for _ in 0..5 {
        gpio_led.set_low();
        Timer::after_millis(500).await;
        gpio_led.set_high();
        Timer::after_millis(500).await;
        info!("Feeding watchdog");
        watchdog.feed(Duration::from_millis(1_050));
    }
    info!("Stopped feeding, device will reset in 1.05 seconds");
    // Blink 10 times per second, not feeding the watchdog.
    // The processor should reset in 1.05 seconds.
    loop {
        gpio_led.set_low();
        Timer::after_millis(100).await;
        gpio_led.set_high();
        Timer::after_millis(100).await;
    }
}
