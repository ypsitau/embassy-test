#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_time::{Duration, Ticker};
use smart_leds::RGB8;
use {defmt_rtt as _, panic_probe as _};

rp::bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => rp::pio::InterruptHandler<rp::peripherals::PIO0>;
    DMA_IRQ_0 => rp::dma::InterruptHandler<rp::peripherals::DMA_CH0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    const NUM_LEDS: usize = 1;
    info!("Start");
    let p = embassy_rp::init(Default::default());
    let mut pio0 = rp::pio::Pio::new(p.PIO0, Irqs);
    let mut rgb8_array = [RGB8::default(); NUM_LEDS];
    let mut ws2812 = {
        let pin = p.PIN_16;
        let program = rp::pio_programs::ws2812::PioWs2812Program::new(&mut pio0.common);
        rp::pio_programs::ws2812::PioWs2812::new(&mut pio0.common, pio0.sm0, p.DMA_CH0, Irqs, pin, &program)
    };
    let mut ticker = Ticker::every(Duration::from_millis(10));
    loop {
        for j in 0..(256 * 5) {
            debug!("New Colors:");
            for i in 0..NUM_LEDS {
                rgb8_array[i] = wheel(((i * 256) / NUM_LEDS + j) & 255);
                debug!("R: {:02x} G: {:02x} B: {:02x}", rgb8_array[i].r, rgb8_array[i].g, rgb8_array[i].b);
            }
            ws2812.write(&rgb8_array).await;
            //ws2812.write_slice(&data).await;
            ticker.next().await;
        }
    }
}

fn wheel(pos: usize) -> RGB8 {
    let pos: u8 = pos as u8;
    if pos < 85 {
        RGB8::new(255 - pos * 3, 0, pos * 3)
    } else if pos < 170 {
        let pos = pos - 85;
        RGB8::new(0, pos * 3, 255 - pos * 3)
    } else {
        let pos = pos - 170;
        RGB8::new(pos * 3, 255 - pos * 3, 0)
    }
}
