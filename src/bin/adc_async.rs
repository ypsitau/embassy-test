#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::adc;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio;
use embassy_rp::Peri;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => adc::InterruptHandler;
});

struct Context<'d> {
    adc: adc::Adc<'d, adc::Async>,
    adc_ch0: adc::Channel<'d>,
    adc_ch1: adc::Channel<'d>,
    adc_ch2: adc::Channel<'d>,
}

impl<'d> Context<'d> {
    fn new(adc: adc::Adc<'d, adc::Async>, pin1: Peri<'d, impl adc::AdcPin>, pin2: Peri<'d, impl adc::AdcPin>, pin3: Peri<'d, impl adc::AdcPin>) -> Self {
        Self {
            adc,
            adc_ch0: adc::Channel::new_pin(pin1, gpio::Pull::None),
            adc_ch1: adc::Channel::new_pin(pin2, gpio::Pull::None),
            adc_ch2: adc::Channel::new_pin(pin3, gpio::Pull::None),
        }
    }
    async fn read_adc(&mut self) -> (u16, u16, u16) {
        let value_ch0 = self.adc.read(&mut self.adc_ch0).await.unwrap();
        let value_ch1 = self.adc.read(&mut self.adc_ch1).await.unwrap();
        let value_ch2 = self.adc.read(&mut self.adc_ch2).await.unwrap();
        (value_ch0, value_ch1, value_ch2)
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut context = Context::new(
        adc::Adc::new(p.ADC, Irqs, Default::default()),
        p.PIN_26,
        p.PIN_27,
        p.PIN_28,
    );
    loop {
        let (value_ch0, value_ch1, value_ch2) = context.read_adc().await;
        info!(
            "ch0: {:03x} ch1: {:03x} ch2: {:03x}",
            value_ch0, value_ch1, value_ch2
        );
        Timer::after(Duration::from_secs(1)).await;
    }
}
