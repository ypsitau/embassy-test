#![no_std]
#![no_main]

use core::cell::RefCell;
use defmt::info;
use embassy_embedded_hal::shared_bus;
use embassy_rp::gpio;
use embedded_graphics as eg;
use embedded_graphics::prelude::*;
use mipidsi::options::{Orientation, Rotation};
use {defmt_rtt as _, panic_probe as _};
//use mipidsi::models::ST7789 as DisplayModel;
use mipidsi::models::ILI9341Rgb565 as DisplayModel;

embassy_rp::bind_interrupts!(struct Irqs {
    DMA_IRQ_0 =>
        embassy_rp::dma::InterruptHandler<embassy_rp::peripherals::DMA_CH0>,
        embassy_rp::dma::InterruptHandler<embassy_rp::peripherals::DMA_CH1>;
});

const SPI_FREQ_DISPLAY: u32 = 64_000_000;
const SPI_FREQ_TOUCH: u32 = 200_000;

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    let p = embassy_rp::init(Default::default());
    info!("Hello World!");
    let pin_spi_clk     = p.PIN_10;
    let pin_spi_mosi    = p.PIN_11;
    let pin_spi_miso    = p.PIN_12;
    let pin_display_rst = p.PIN_6;
    let pin_display_dc  = p.PIN_7;
    let pin_display_cs  = p.PIN_8;
    let pin_display_bl  = p.PIN_9;
    let pin_touch_cs    = p.PIN_14;
    let _pin_touch_irq  = p.PIN_15;

    let spi_bus_shared = {
        //let spi_bus = embassy_rp::spi::Spi::new_blocking(p.SPI1, pin_spi_clk, pin_spi_mosi, pin_spi_miso, Default::default());
        let spi_bus = embassy_rp::spi::Spi::new(p.SPI1, pin_spi_clk, pin_spi_mosi, pin_spi_miso, p.DMA_CH0, p.DMA_CH1, Irqs, Default::default());
        embassy_sync::blocking_mutex::Mutex::<embassy_sync::blocking_mutex::raw::NoopRawMutex, _>::new(RefCell::new(spi_bus))
    };
    let mut touch = {
        let spi_device = {
            let mut config = embassy_rp::spi::Config::default();
            config.frequency = SPI_FREQ_TOUCH;
            config.phase = embassy_rp::spi::Phase::CaptureOnSecondTransition;
            config.polarity = embassy_rp::spi::Polarity::IdleHigh;
            shared_bus::blocking::spi::SpiDeviceWithConfig::new(
                &spi_bus_shared, gpio::Output::new(pin_touch_cs, gpio::Level::High), config)
        };
        xpt2046::Driver::new(spi_device)
    };
    let mut spi_buf = [0u8; 320];
    let mut draw_target = {
        let gpio_dc = gpio::Output::new(pin_display_dc, gpio::Level::Low);
        let gpio_rst = gpio::Output::new(pin_display_rst, gpio::Level::Low);
        let spi_device = {
            let mut config = embassy_rp::spi::Config::default();
            config.frequency = SPI_FREQ_DISPLAY;
            config.phase = embassy_rp::spi::Phase::CaptureOnSecondTransition;
            config.polarity = embassy_rp::spi::Polarity::IdleHigh;
            shared_bus::blocking::spi::SpiDeviceWithConfig::new(
                &spi_bus_shared, gpio::Output::new(pin_display_cs, gpio::Level::High), config)
        };
        let display_interface = mipidsi::interface::SpiInterface::new(spi_device, gpio_dc, &mut spi_buf);
        mipidsi::Builder::new(DisplayModel, display_interface)
            .display_size(240, 320)
            .reset_pin(gpio_rst)
            .orientation(Orientation::new().rotate(Rotation::Deg90).flip_horizontal())
            .init(&mut embassy_time::Delay)
            .unwrap()
    };
    let _gpio_bl = gpio::Output::new(pin_display_bl, gpio::Level::High);
    draw_target.clear(eg::pixelcolor::Rgb565::BLACK).unwrap();
    eg::image::Image::new(
        &eg::image::ImageRawLE::new(include_bytes!("../../assets/ferris.raw"), 86),
        Point::new(34, 68)
    ).draw(&mut draw_target).unwrap();
    let text_style = eg::mono_font::MonoTextStyleBuilder::new()
        .font(&eg::mono_font::ascii::FONT_10X20)
        .text_color(eg::pixelcolor::Rgb565::GREEN)
        .build();
    eg::text::Text::new(
        "Hello embedded_graphics \n + embassy + RP2040!",
        Point::new(20, 200),
        text_style
    ).draw(&mut draw_target).unwrap();
    let style_dot = eg::primitives::PrimitiveStyleBuilder::new()
        .fill_color(eg::pixelcolor::Rgb565::BLUE)
        .build();
    loop {
        if let Some((x, y)) = touch.read() {
            eg::primitives::Rectangle::new(
                Point::new(x - 1, y - 1),
                Size::new(3, 3)
            ).into_styled(style_dot).draw(&mut draw_target).unwrap();
        }
    }
}

mod xpt2046 {
    use embedded_hal_1::spi;
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

    pub struct Driver<SPI> {
        spi: SPI,
    }

    impl<SPI: spi::SpiDevice> Driver<SPI> {
        pub fn new(spi: SPI) -> Self {
            Self { spi }
        }

        pub fn read(&mut self) -> Option<(i32, i32)> {
            let mut xbytes = [0u8; 2];
            let mut ybytes = [0u8; 2];
            self.spi.transaction(&mut [
                spi::Operation::Write(&[0x90]),
                spi::Operation::Read(&mut xbytes),
                spi::Operation::Write(&[0xd0]),
                spi::Operation::Read(&mut ybytes),
            ]).unwrap();
            let xraw = (u16::from_be_bytes(xbytes) >> 3) as i32;
            let yraw = (u16::from_be_bytes(ybytes) >> 3) as i32;
            CALIBRATION.calc_pos(xraw, yraw)
        }
    }
}
