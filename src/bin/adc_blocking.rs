#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

struct Context<'d> {
    adc: rp::adc::Adc<'d, rp::adc::Blocking>,
    adc_ch0: rp::adc::Channel<'d>,
    adc_ch1: rp::adc::Channel<'d>,
    adc_ch2: rp::adc::Channel<'d>,
}

impl<'d> Context<'d> {
    fn new(adc: rp::adc::Adc<'d, rp::adc::Blocking>, pin1: rp::Peri<'d, impl rp::adc::AdcPin>, pin2: rp::Peri<'d, impl rp::adc::AdcPin>, pin3: rp::Peri<'d, impl rp::adc::AdcPin>) -> Self {
        Self {
            adc,
            adc_ch0: rp::adc::Channel::new_pin(pin1, rp::gpio::Pull::None),
            adc_ch1: rp::adc::Channel::new_pin(pin2, rp::gpio::Pull::None),
            adc_ch2: rp::adc::Channel::new_pin(pin3, rp::gpio::Pull::None),
        }
    }
    fn read_adc(&mut self) -> (u16, u16, u16) {
        let value_ch0 = self.adc.blocking_read(&mut self.adc_ch0).unwrap();
        let value_ch1 = self.adc.blocking_read(&mut self.adc_ch1).unwrap();
        let value_ch2 = self.adc.blocking_read(&mut self.adc_ch2).unwrap();
        (value_ch0, value_ch1, value_ch2)
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    let mut context = Context::new(
        rp::adc::Adc::new_blocking(p.ADC, Default::default()),
        p.PIN_26,
        p.PIN_27,
        p.PIN_28,
    );
    loop {
        let (value_ch0, value_ch1, value_ch2) = context.read_adc();
        info!(
            "ch0: {:03x} ch1: {:03x} ch2: {:03x}",
            value_ch0, value_ch1, value_ch2
        );
        Timer::after(Duration::from_secs(1)).await;
    }
}
