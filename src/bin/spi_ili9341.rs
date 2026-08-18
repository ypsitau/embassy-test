#![no_std]
#![no_main]

use core::cell::RefCell;
use embassy_time as time;
use embassy_futures as futures;
use embassy_embedded_hal as hal;
use embassy_rp as rp;
use embedded_graphics as eg;
use embedded_graphics::prelude::*;
use mipidsi::options::{Orientation, Rotation};
//use mipidsi::models::ST7789 as DisplayModel;
use mipidsi::models::ILI9341Rgb565 as DisplayModel;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

//rp::bind_interrupts!(struct Irqs {
//    DMA_IRQ_0 =>
//        rp::dma::InterruptHandler<rp::peripherals::DMA_CH0>,
//        rp::dma::InterruptHandler<rp::peripherals::DMA_CH1>;
//});
//
//type MutexSPI1 = embassy_sync::blocking_mutex::Mutex<
//    embassy_sync::blocking_mutex::raw::NoopRawMutex,
//    RefCell<rp::spi::Spi<'static, rp::peripherals::SPI1, rp::spi::Async>>>;

#[derive(Debug, Clone, Copy)]
struct Pos {
    x: i32,
    y: i32,
}

type MutexPos = embassy_sync::blocking_mutex::Mutex<
    embassy_sync::blocking_mutex::raw::NoopRawMutex,
    RefCell<Option<Pos>>>;

type MutexSPI1 = embassy_sync::blocking_mutex::Mutex<
    embassy_sync::blocking_mutex::raw::NoopRawMutex,
    RefCell<rp::spi::Spi<'static, rp::peripherals::SPI1, rp::spi::Blocking>>>;

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
    let mut touch = {
        let spi_device = {
            let gpio_cs = rp::gpio::Output::new(p.PIN_14, rp::gpio::Level::High);
            let _pin_touch_irq  = p.PIN_15;
            let mut config = rp::spi::Config::default();
            config.frequency = 200_000;
            config.phase = rp::spi::Phase::CaptureOnSecondTransition;
            config.polarity = rp::spi::Polarity::IdleHigh;
            hal::shared_bus::blocking::spi::SpiDeviceWithConfig::new(&mutex_spi, gpio_cs, config)
        };
        xpt2046::Driver::new(spi_device)
    };
    let mut display = {
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
        .fill_color(eg::pixelcolor::Rgb565::BLUE)
        .build();
    let mutex_pos = MutexPos::new(RefCell::new(None));
    let fut_main = async {
        loop {
            mutex_pos.lock(|pos| {
                if let Some(pos) = *pos.borrow() {
                    eg::primitives::Rectangle::new(
                        Point::new(pos.x - 1, pos.y - 1),
                        Size::new(3, 3)
                    ).into_styled(style_dot).draw(&mut display).unwrap();
                }
            });
            time::Timer::after_millis(100).await;
        }
    };
    let fut_touch = async {
        loop {
            if let Some((x, y)) = touch.read_pos() {
                mutex_pos.lock(|pos| {
                    *pos.borrow_mut() = Some(Pos { x, y });
                });
            } else {
                mutex_pos.lock(|pos| {
                    *pos.borrow_mut() = None;
                });
            }
            time::Timer::after_millis(10).await;
        }
    };
    futures::join::join(fut_main, fut_touch).await;
}

mod xpt2046 {
    use embedded_hal_1 as hal;
    struct Calibration {
        xraw_max: i32,
        xraw_min: i32,
        yraw_min: i32,
        yraw_max: i32,
        x_range: i32,
        y_range: i32,
    }
    impl Calibration {
        fn calc_pos(&self, xraw: i32, yraw: i32) -> Option<(i32, i32)> {
            let x = ((xraw - self.xraw_min) * self.x_range / (self.xraw_max - self.xraw_min)).clamp(0, self.x_range);
            let y = ((yraw - self.yraw_min) * self.y_range / (self.yraw_max - self.yraw_min)).clamp(0, self.y_range);
            if x == 0 && y == 0 { None } else { Some((x, y)) }
        }
    }
    const CALIBRATION: Calibration = Calibration {
        xraw_min: 340,
        xraw_max: 3880,
        yraw_min: 262,
        yraw_max: 3850,
        x_range: 320,
        y_range: 240,
    };
    pub struct Driver<SpiDevice> {
        spi_device: SpiDevice,
    }
    impl<SpiDevice: hal::spi::SpiDevice> Driver<SpiDevice> {
        pub fn new(spi_device: SpiDevice) -> Self {
            Self { spi_device }
        }
        pub fn read_pos(&mut self) -> Option<(i32, i32)> {
            let mut xbytes = [0u8; 2];
            let mut ybytes = [0u8; 2];
            self.spi_device.transaction(&mut [
                hal::spi::Operation::Write(&[0x90]),
                hal::spi::Operation::Read(&mut xbytes),
                hal::spi::Operation::Write(&[0xd0]),
                hal::spi::Operation::Read(&mut ybytes),
            ]).unwrap();
            let xraw = (u16::from_be_bytes(xbytes) >> 3) as i32;
            let yraw = (u16::from_be_bytes(ybytes) >> 3) as i32;
            CALIBRATION.calc_pos(xraw, yraw)
        }
    }
}
