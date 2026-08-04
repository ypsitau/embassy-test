#![no_std]
#![no_main]

use defmt::{expect, info};
use embassy_executor::Spawner;
use embassy_rp::adc;
use embassy_rp::gpio;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut adc = adc::Adc::new_blocking(p.ADC, Default::default());
    let mut adc_ch0 = adc::Channel::new_pin(p.PIN_26, gpio::Pull::None);
    let mut adc_ch1 = adc::Channel::new_pin(p.PIN_27, gpio::Pull::None);
    let mut adc_ch2 = adc::Channel::new_pin(p.PIN_28, gpio::Pull::None);
    loop {
        let value_ch0 = expect!(adc.blocking_read(&mut adc_ch0));
        let value_ch1 = expect!(adc.blocking_read(&mut adc_ch1));
        let value_ch2 = expect!(adc.blocking_read(&mut adc_ch2));
        info!(
            "ch0: {:03x} ch1: {:03x} ch2: {:03x}",
            value_ch0, value_ch1, value_ch2
        );
        Timer::after_secs(1).await;
    }
}
