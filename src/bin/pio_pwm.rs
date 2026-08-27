#![no_std]
#![no_main]

use core::time::Duration;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_rp::peripherals;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

rp::bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => rp::pio::InterruptHandler<peripherals::PIO0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> !{
    let p = embassy_rp::init(Default::default());
    let mut pio0 = rp::pio::Pio::new(p.PIO0, Irqs);
    let mut pwm = {
        let pin = p.PIN_25;
        let program = rp::pio_programs::pwm::PioPwmProgram::new(&mut pio0.common);
        rp::pio_programs::pwm::PioPwm::new(&mut pio0.common, pio0.sm0, pin, &program)
    };
    pwm.set_period(Duration::from_micros(20_000));
    pwm.start();
    let mut duration = 0;
    loop {
        duration = (duration + 1) % 1000;
        pwm.write(Duration::from_micros(duration));
        Timer::after_millis(1).await;
    }
}
