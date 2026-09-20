#![no_std]
#![no_main]
mod emb {
    pub use embassy_executor as executor;
    pub use embassy_futures as futures;
    pub use embassy_embedded_hal as hal;
    pub use embassy_sync as sync;
    pub use embassy_time as time;
}

use embedded_hal_1 as hal;
use embedded_graphics as eg;

use core::cell::RefCell;
use defmt::info;
//use mipidsi::models::ST7789 as DisplayModel;
use mipidsi::models::ILI9341Rgb565 as DisplayModel;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use embassy_test::xpt2046;

use embassy_rp as rp;

//rp::bind_interrupts!(struct Irqs {
//    DMA_IRQ_0 =>
//        rp::dma::InterruptHandler<rp::peripherals::DMA_CH0>,
//        rp::dma::InterruptHandler<rp::peripherals::DMA_CH1>;
//});
//
//type MutexSPI1 = emb::sync::blocking_mutex::Mutex<
//    emb::sync::blocking_mutex::raw::NoopRawMutex,
//    RefCell<rp::spi::Spi<'static, rp::peripherals::SPI1, rp::spi::Async>>>;

#[emb::executor::main]
async fn main(_spawner: emb::executor::Spawner) {
    let (spi_touch, spi_display, pin_display_reset, pin_display_dc) = {
        let p = rp::init(Default::default());
        let mutex_spi = {
            type MutexSPI1 = emb::sync::blocking_mutex::Mutex<
                emb::sync::blocking_mutex::raw::NoopRawMutex,
                RefCell<rp::spi::Spi<'static, rp::peripherals::SPI1, rp::spi::Blocking>>>;
            let pin_clk = p.PIN_10;
            let pin_mosi = p.PIN_11;
            let pin_miso = p.PIN_12;
            let config = rp::spi::Config::default();
            //let tx_dma = p.DMA_CH0;
            //let rx_dma = p.DMA_CH1;
            //let spi = rp::spi::Spi::new(p.SPI1, pin_clk, pin_mosi, pin_miso, tx_dma, rx_dma, Irqs, config);
            let spi = rp::spi::Spi::new_blocking(p.SPI1, pin_clk, pin_mosi, pin_miso, config);
            static STATIC_CELL: StaticCell<MutexSPI1> = StaticCell::new();
            STATIC_CELL.init(MutexSPI1::new(RefCell::new(spi)))
        };
        let spi_touch = {
            let pin_cs = rp::gpio::Output::new(p.PIN_14, rp::gpio::Level::High);
            let _pin_touch_irq  = p.PIN_15;
            let mut config = rp::spi::Config::default();
            config.frequency = 200_000;
            config.phase = rp::spi::Phase::CaptureOnSecondTransition;
            config.polarity = rp::spi::Polarity::IdleHigh;
            emb::hal::shared_bus::blocking::spi::SpiDeviceWithConfig::new(mutex_spi, pin_cs, config)
        };
        let spi_display = {
            let pin_cs = rp::gpio::Output::new(p.PIN_8, rp::gpio::Level::High);
            let mut config = rp::spi::Config::default();
            config.frequency = 64_000_000;
            config.phase = rp::spi::Phase::CaptureOnSecondTransition;
            config.polarity = rp::spi::Polarity::IdleHigh;
            emb::hal::shared_bus::blocking::spi::SpiDeviceWithConfig::new(mutex_spi, pin_cs, config)
        };
        let pin_display_reset = rp::gpio::Output::new(p.PIN_6, rp::gpio::Level::Low);
        let pin_display_dc = rp::gpio::Output::new(p.PIN_7, rp::gpio::Level::Low);
        let _pin_display_bl = {
            static STATIC_CELL: StaticCell<rp::gpio::Output<'static>> = StaticCell::new();
            STATIC_CELL.init(rp::gpio::Output::new(p.PIN_9, rp::gpio::Level::High));
        };
        (spi_touch, spi_display, pin_display_reset, pin_display_dc)
    };
    let fut_task_main = task_main(spi_touch, spi_display, pin_display_reset, pin_display_dc);
    fut_task_main.await;
}

async fn task_main(spi_touch: impl hal::spi::SpiDevice, spi_display: impl hal::spi::SpiDevice,
        pin_display_reset: impl hal::digital::OutputPin, pin_display_dc: impl hal::digital::OutputPin) {
    use eg::prelude::*;
    let mut touch = xpt2046::Builder::new(spi_touch, 240, 320)
        .calibration(xpt2046::Calibration::default())
        .rotate90(true)
        .build();
    let mut display = {
        use mipidsi::options::{Orientation, Rotation, ColorOrder};
        let display_interface = {
            let spi_buf = {
                const SPI_BUF_SIZE: usize = 320;
                static STATIC_CELL: StaticCell<[u8; SPI_BUF_SIZE]> = StaticCell::new();
                STATIC_CELL.init([0u8; SPI_BUF_SIZE])
            };
            mipidsi::interface::SpiInterface::new(spi_display, pin_display_dc, spi_buf)
        };
        mipidsi::Builder::new(DisplayModel, display_interface)
            .display_size(240, 320)
            .color_order(ColorOrder::Bgr)
            .reset_pin(pin_display_reset)
            .orientation(Orientation::new().rotate(Rotation::Deg90).flip_horizontal())
            .init(&mut emb::time::Delay)
            .unwrap()
    };
    if let Some(calibration) = xpt2046::calibrate(&mut touch, &mut display, &mut emb::time::Delay,
            eg::pixelcolor::Rgb565::GREEN, eg::pixelcolor::Rgb565::BLACK).await {
        info!("touch.calibration = xpt2046::Calibration::new({}, {}, {}, {});",
            calibration.xraw_right, calibration.xraw_left, calibration.yraw_top, calibration.yraw_bottom);
        touch.calibration = calibration;
    }
    //touch.calibration = xpt2046::Calibration::new(1887, 202, 132, 1853);
    display.clear(eg::pixelcolor::Rgb565::BLACK).unwrap();
    eg::image::Image::new(
        &eg::image::ImageRawLE::new(include_bytes!("../../assets/ferris.raw"), 86),
        Point::new(34, 68)
    ).draw(&mut display).unwrap();
    let text_style = eg::mono_font::MonoTextStyleBuilder::new()
        .font(&eg::mono_font::ascii::FONT_10X20)
        .text_color(eg::pixelcolor::Rgb565::GREEN)
        .build();
    eg::text::Text::new(
        "Hello embedded_graphics \n + embassy + RP2040!",
        Point::new(20, 200),
        text_style
    ).draw(&mut display).unwrap();
    let channel = emb::sync::channel::Channel::<
        emb::sync::blocking_mutex::raw::NoopRawMutex, (i32, i32), 16>::new();
    let fut_main = async {
        let style_dot = eg::primitives::PrimitiveStyleBuilder::new()
            .fill_color(eg::pixelcolor::Rgb565::WHITE)
            .build();
        loop {
            let (x, y) = channel.receive().await;
            eg::primitives::Rectangle::new(
                Point::new(x - 4, y - 4), Size::new(8, 8)
            ).into_styled(style_dot).draw(&mut display).unwrap();
        }
    };
    let fut_touch = touch.run_sampler(emb::time::Delay, 5, |pos| {
        if let Some(pos) = pos { channel.try_send(pos).ok(); }
    });
    emb::futures::join::join(fut_main, fut_touch).await;
}
