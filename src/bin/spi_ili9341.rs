#![no_std]
#![no_main]

use core::cell::RefCell;
use embassy_time as time;
use embassy_futures as futures;
use embassy_embedded_hal as hal;
use embassy_rp as rp;
use embedded_graphics as eg;
use embedded_graphics::prelude::*;
//use mipidsi::models::ST7789 as DisplayModel;
use mipidsi::models::ILI9341Rgb565 as DisplayModel;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use embassy_test::xpt2046;

//rp::bind_interrupts!(struct Irqs {
//    DMA_IRQ_0 =>
//        rp::dma::InterruptHandler<rp::peripherals::DMA_CH0>,
//        rp::dma::InterruptHandler<rp::peripherals::DMA_CH1>;
//});
//
//type MutexSPI1 = embassy_sync::blocking_mutex::Mutex<
//    embassy_sync::blocking_mutex::raw::NoopRawMutex,
//    RefCell<rp::spi::Spi<'static, rp::peripherals::SPI1, rp::spi::Async>>>;

type MutexSPI1 = embassy_sync::blocking_mutex::Mutex<
    embassy_sync::blocking_mutex::raw::NoopRawMutex,
    RefCell<rp::spi::Spi<'static, rp::peripherals::SPI1, rp::spi::Blocking>>>;

type MutexPos = embassy_sync::blocking_mutex::Mutex<
    embassy_sync::blocking_mutex::raw::NoopRawMutex,
    RefCell<Option<xpt2046::Pos>>>;

struct SharedPos {
    mutex_pos: MutexPos,
}

impl SharedPos {
    fn new() -> Self {
        Self {
            mutex_pos: MutexPos::new(RefCell::new(None)),
        }
    }
}

impl xpt2046::SharedPos for SharedPos {
    fn get_pos(&self) -> Option<xpt2046::Pos> {
        self.mutex_pos.lock(|p| *p.borrow())
    }
    fn set_pos(&self, pos: Option<xpt2046::Pos>) {
        self.mutex_pos.lock(|p| {
            *p.borrow_mut() = pos;
        });
    }
}

use xpt2046::SharedPos as _;

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    let p = rp::init(Default::default());
    let mutex_spi = {
        let clk = p.PIN_10;
        let mosi = p.PIN_11;
        let miso = p.PIN_12;
        let config = rp::spi::Config::default();
        //let tx_dma = p.DMA_CH0;
        //let rx_dma = p.DMA_CH1;
        //let spi = rp::spi::Spi::new(p.SPI1, clk, mosi, miso, tx_dma, rx_dma, Irqs, config);
        let spi = rp::spi::Spi::new_blocking(p.SPI1, clk, mosi, miso, config);
        MutexSPI1::new(RefCell::new(spi))
    };
    let shared_pos = SharedPos::new();
    let fut_touch = {
        let spi_device = {
            let gpio_cs = rp::gpio::Output::new(p.PIN_14, rp::gpio::Level::High);
            let _pin_touch_irq  = p.PIN_15;
            let mut config = rp::spi::Config::default();
            config.frequency = 200_000;
            config.phase = rp::spi::Phase::CaptureOnSecondTransition;
            config.polarity = rp::spi::Polarity::IdleHigh;
            hal::shared_bus::blocking::spi::SpiDeviceWithConfig::new(&mutex_spi, gpio_cs, config)
        };
        xpt2046::task(spi_device, &shared_pos, embassy_time::Delay)
    };
    let mut display = {
        use mipidsi::options::{Orientation, Rotation, ColorOrder};
        let gpio_rst = rp::gpio::Output::new(p.PIN_6, rp::gpio::Level::Low);
        let gpio_dc = rp::gpio::Output::new(p.PIN_7, rp::gpio::Level::Low);
        let gpio_cs = rp::gpio::Output::new(p.PIN_8, rp::gpio::Level::High);
        static GPIO_BL: StaticCell<rp::gpio::Output<'static>> = StaticCell::new();
        let _gpio_bl = GPIO_BL.init(rp::gpio::Output::new(p.PIN_9, rp::gpio::Level::High));
        let spi_device = {
            let mut config = rp::spi::Config::default();
            config.frequency = 64_000_000;
            config.phase = rp::spi::Phase::CaptureOnSecondTransition;
            config.polarity = rp::spi::Polarity::IdleHigh;
            hal::shared_bus::blocking::spi::SpiDeviceWithConfig::new(&mutex_spi, gpio_cs, config)
        };
        static SPI_BUF: StaticCell<[u8; 320]> = StaticCell::new();
        let spi_buf = SPI_BUF.init([0u8; 320]);
        let display_interface = mipidsi::interface::SpiInterface::new(spi_device, gpio_dc, spi_buf);
        mipidsi::Builder::new(DisplayModel, display_interface)
            .display_size(240, 320)
            .color_order(ColorOrder::Bgr)
            .reset_pin(gpio_rst)
            .orientation(Orientation::new().rotate(Rotation::Deg90).flip_horizontal())
            .init(&mut embassy_time::Delay)
            .unwrap()
    };
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
    let style_dot = eg::primitives::PrimitiveStyleBuilder::new()
        .fill_color(eg::pixelcolor::Rgb565::WHITE)
        .build();
    let fut_main = async {
        loop {
            if let Some(pos) = shared_pos.get_pos() {
                eg::primitives::Rectangle::new(
                    Point::new(pos.x - 4, pos.y - 4),
                    Size::new(8, 8)
                ).into_styled(style_dot).draw(&mut display).unwrap();
            }
            time::Timer::after_millis(30).await;
        }
    };
    futures::join::join(fut_main, fut_touch).await;
}
