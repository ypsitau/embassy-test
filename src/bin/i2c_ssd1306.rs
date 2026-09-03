#![no_std]
#![no_main]

use core::fmt::Write;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_time::Timer;
use embedded_graphics as eg;
use embedded_graphics::prelude::*;
use heapless::String;
use ssd1306::prelude::*;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

enum Direction {
    Inc,
    Dec,
}

impl Direction {
    fn step(&self, value: &mut i32) {
        *value += match self {
            Direction::Inc => 1,
            Direction::Dec => -1,
        };
    }
}

type MutexI2C0 = embassy_sync::mutex::Mutex<
    embassy_sync::blocking_mutex::raw::NoopRawMutex,
    rp::i2c::I2c<'static, rp::peripherals::I2C0, rp::i2c::Async>>;

rp::bind_interrupts!(struct Irqs {
    I2C0_IRQ => rp::i2c::InterruptHandler<rp::peripherals::I2C0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    let mutex_i2c0 = {
        let sda = p.PIN_16;
        let scl = p.PIN_17;
        let mut config = rp::i2c::Config::default();
        config.frequency = 400_000;
        // should be replaced by make_static macro when it becomes available
        static STATIC_CELL: StaticCell<MutexI2C0> = StaticCell::new();
        STATIC_CELL.init(MutexI2C0::new(rp::i2c::I2c::new_async(p.I2C0, scl, sda, Irqs, config)))
    };
    let mut display = {
        // impl embedded_hal::i2c::I2c
        let i2c_device = embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice::new(mutex_i2c0);
        let interface = ssd1306::I2CDisplayInterface::new(i2c_device);
        ssd1306::Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode()
    };
    display.init().await.unwrap();
    let text_style = eg::mono_font::MonoTextStyleBuilder::new()
        .font(&eg::mono_font::ascii::FONT_10X20)
        .text_color(eg::pixelcolor::BinaryColor::On)
        .build();
    let mut text: String<16> = String::new();
    let style_dot = eg::primitives::PrimitiveStyleBuilder::new()
        .fill_color(eg::pixelcolor::BinaryColor::On)
        .build();
    let size_display = display.dimensions();
    let size_display = Size::new(size_display.0 as u32, size_display.1 as u32);
    let size_dot = Size::new(8, 8);
    let mut i = 0;
    let mut x = size_display.width as i32 / 2;
    let mut y = size_display.height as i32 / 2;
    let mut x_dir = Direction::Inc;
    let mut y_dir = Direction::Inc;
    loop {
        display.clear(eg::pixelcolor::BinaryColor::Off).unwrap();
        text.clear();
        write!(text, "Frame: {}", i).unwrap();
        eg::text::Text::with_baseline(
            text.as_str(),
            Point::new(0, 0),
            text_style,
            eg::text::Baseline::Top
        ).draw(&mut display).unwrap();
        eg::primitives::Rectangle::new(
            Point::new(x, y),
            size_dot
        ).into_styled(style_dot).draw(&mut display).unwrap();
        Timer::after_millis(10).await;
        i += 1;
        if x >= (size_display.width - size_dot.width) as i32 {
            x_dir = Direction::Dec;
        } else if x <= 0 {
            x_dir = Direction::Inc;
        }
        if y >= (size_display.height - size_dot.height) as i32 {
            y_dir = Direction::Dec;
        } else if y <= 0 {
            y_dir = Direction::Inc;
        }
        x_dir.step(&mut x);
        y_dir.step(&mut y);
        display.flush().await.unwrap();
    }
}
