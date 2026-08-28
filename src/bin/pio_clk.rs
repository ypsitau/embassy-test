//! This example shows how to output a clock signal on an output pin using the PIO module in the RP2040 chip.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

rp::bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => rp::pio::InterruptHandler<rp::peripherals::PIO0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut pio0 = rp::pio::Pio::new(p.PIO0, Irqs);
    let mut clk = {
        let pin = p.PIN_18;
        let frequency = 10_000; // 10 kHz
        let program = rp::pio_programs::clk::PioClkProgram::new(&mut pio0.common);
        rp::pio_programs::clk::PioClk::new(&mut pio0.common, pio0.sm0, pin, &program, frequency)
    };
    loop {
        clk.start();
        Timer::after_millis(5000).await;
        clk.stop();
        Timer::after_millis(5000).await;
    }
}
