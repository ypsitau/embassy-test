#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::gpio;
use embassy_rp::Peri;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

struct Context<'d> {
    gpio_led: gpio::Output<'d>,
    gpio_button: gpio::Input<'d>,
}

impl<'d> Context<'d> {
    fn new(pin_led: Peri<'d, impl gpio::Pin>, pin_button: Peri<'d, impl gpio::Pin>) -> Self {
        Self {
            gpio_led: gpio::Output::new(pin_led, gpio::Level::Low),
            gpio_button: gpio::Input::new(pin_button, gpio::Pull::Up),
        }
    }
    fn get_button_state(&self) -> bool {
        self.gpio_button.is_low()
    }
    fn set_led_high(&mut self) {
        self.gpio_led.set_high();
    }
    fn set_led_low(&mut self) {
        self.gpio_led.set_low();
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut context = Context::new(p.PIN_25, p.PIN_15);
    let mut state_prev = context.get_button_state();
    loop {
        let state = context.get_button_state();
        if state == state_prev {
            // no change
        } else if state {
            info!("button pressed!");
            context.set_led_high();
        } else {
            info!("button released!");
            context.set_led_low();
        }
        state_prev = state;
        Timer::after_millis(100).await;
    }
}
