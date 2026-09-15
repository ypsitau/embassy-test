#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::timer::timg::TimerGroup;
use esp_hal as esp;

esp_bootloader_esp_idf::esp_app_desc!();

async fn blinky() {
    loop {
        esp_println::println!("Hello world from embassy!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

#[esp_rtos::main]
async fn main(_spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let p = esp_hal::init(esp_hal::Config::default());

    esp_println::println!("Init!");

    let timg0 = TimerGroup::new(p.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(p.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);
    let mut pin_led = esp::gpio::Output::new(p.GPIO4, esp::gpio::Level::Low, esp::gpio::OutputConfig::default());
    let fut_blinky = blinky();
    let fut_main = async {
        loop {
            pin_led.toggle();
            esp_println::println!("Bing!");
            Timer::after(Duration::from_millis(5_000)).await;
        }
    };
    embassy_futures::join::join(fut_main, fut_blinky).await;
}

/*
use esp_hal as hal;
use panic_halt as _;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(clippy::large_stack_frames, reason = "esp_hal demands large stack frames in main")]
#[hal::main]
fn main() -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32 -o esp32-wroom-32e
    let (mut pin_led,) = {
        let config = hal::Config::default().with_cpu_clock(hal::clock::CpuClock::max());
        let p = hal::init(config);
        // The following pins are used to bootstrap the chip. They are available
        // for use, but check the datasheet of the module for more information on them.
        // - GPIO0
        // - GPIO2
        // - GPIO5
        // - GPIO12
        // - GPIO15
        // These GPIO pins are in use by some feature of the module and should not be used.
        let _ = p.GPIO6;
        let _ = p.GPIO7;
        let _ = p.GPIO8;
        let _ = p.GPIO9;
        let _ = p.GPIO10;
        let _ = p.GPIO11;
        let _ = p.GPIO16;
        let _ = p.GPIO20;
        let pin_led = hal::gpio::Output::new(p.GPIO4, hal::gpio::Level::Low, hal::gpio::OutputConfig::default());
        (pin_led,)
    };
    loop {
        pin_led.toggle();
        let delay_start = hal::time::Instant::now();
        while delay_start.elapsed() < hal::time::Duration::from_millis(500) {}
    }
}
*/
