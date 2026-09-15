#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time as time;
use embedded_hal_1 as hal;
use embedded_hal_async as hal_async;
use esp_backtrace as _;
use esp_hal as esp;
use esp_println::println as info;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(_spawner: Spawner) {
    let (pin_sw, pin_led) = {
        let p = esp_hal::init(esp_hal::Config::default());
        esp_println::logger::init_logger_from_env();
        let timg0 = esp::timer::timg::TimerGroup::new(p.TIMG0);
        let software_interrupt_control = esp::interrupt::software::SoftwareInterruptControl::new(p.SW_INTERRUPT);
        esp_rtos::start(timg0.timer0, software_interrupt_control.software_interrupt0);
        let pin_sw = {
            let config = esp::gpio::InputConfig::default().with_pull(esp::gpio::Pull::Up);
            esp::gpio::Input::new(p.GPIO0, config)
        };
        let pin_led = {
            esp::gpio::Output::new(p.GPIO4, esp::gpio::Level::Low, esp::gpio::OutputConfig::default())
        };
        (pin_sw, pin_led)
    };
    let fut_blinky = blinky(pin_sw, pin_led);
    let fut_main = async {
        loop {
            info!("Bing!");
            time::Timer::after_millis(5000).await;
        }
    };
    embassy_futures::join::join(fut_main, fut_blinky).await;
}

async fn blinky(mut pin_sw: impl hal::digital::InputPin + hal_async::digital::Wait, mut pin_led: impl hal::digital::OutputPin) {
    loop {
        let is_pushed = pin_sw.is_low().unwrap_or(false);
        if is_pushed {
            pin_led.set_high().ok();
        } else {
            pin_led.set_low().ok();
        }
        pin_sw.wait_for_any_edge().await.ok();
        time::Timer::after_millis(30).await;    // Debounce delay
    }
}
