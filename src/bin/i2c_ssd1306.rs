#![no_std]
#![no_main]

use core::fmt::Write;
use embassy_executor::Spawner;
use embassy_rp::{i2c, peripherals};
use embassy_time::Timer;
use embedded_graphics as eg;
use embedded_graphics::prelude::*;
use heapless::String;
use ssd1306::prelude::*;
use {defmt_rtt as _, panic_probe as _};

embassy_rp::bind_interrupts!(struct Irqs {
    I2C0_IRQ => i2c::InterruptHandler<peripherals::I2C0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let i2c0 = {
        let sda = p.PIN_16;
        let scl = p.PIN_17;
        let mut config = i2c::Config::default();
        config.frequency = 400_000;
        i2c::I2c::new_async(p.I2C0, scl, sda, Irqs, config)
    };
    let mut draw_target = {
        let interface = ssd1306::I2CDisplayInterface::new(i2c0);
        ssd1306::Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode()
    };
    draw_target.init().await.unwrap();
    let text_style = eg::mono_font::MonoTextStyleBuilder::new()
        .font(&eg::mono_font::ascii::FONT_10X20)
        .text_color(eg::pixelcolor::BinaryColor::On)
        .build();
    draw_target.clear(eg::pixelcolor::BinaryColor::Off).unwrap();
    for i in 0..=1 {
        let mut text: String<16> = String::new();
        write!(text, "line.{}", i + 1).unwrap();
        eg::text::Text::with_baseline(
            text.as_str(),
            eg::prelude::Point::new(0, i * 20),
            text_style,
            eg::text::Baseline::Top,
        )
        .draw(&mut draw_target)
        .unwrap();
    }
    draw_target.flush().await.unwrap();
    loop {
        Timer::after_millis(1000).await;
    }
}
