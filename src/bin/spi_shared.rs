//! This example shows how to share (async) I2C and SPI buses between multiple devices.

#![no_std]
#![no_main]

use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

type Spi1Bus = Mutex<NoopRawMutex, rp::spi::Spi<'static, rp::peripherals::SPI1, rp::spi::Async>>;

rp::bind_interrupts!(struct Irqs {
    DMA_IRQ_0 =>
        rp::dma::InterruptHandler<rp::peripherals::DMA_CH0>,
        rp::dma::InterruptHandler<rp::peripherals::DMA_CH1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = rp::init(Default::default());
    let spi_bus = {
        let clk = p.PIN_10;
        let mosi = p.PIN_11;
        let miso = p.PIN_12;
        let tx_dma = p.DMA_CH0;
        let rx_dma = p.DMA_CH1;
        let config = rp::spi::Config::default();
        let spi = rp::spi::Spi::new(p.SPI1, clk, mosi, miso, tx_dma, rx_dma, Irqs, config);
        static SPI_BUS: StaticCell<Spi1Bus> = StaticCell::new();
        SPI_BUS.init(Mutex::new(spi))
    };
    spawner.spawn(task_a(spi_bus, rp::gpio::Output::new(p.PIN_0, rp::gpio::Level::High)).unwrap());
    spawner.spawn(task_b(spi_bus, rp::gpio::Output::new(p.PIN_1, rp::gpio::Level::High)).unwrap());
}

#[embassy_executor::task]
async fn task_a(spi_bus: &'static Spi1Bus, cs: rp::gpio::Output<'static>) {
    let spi_dev = SpiDevice::new(spi_bus, cs);
    let _sensor = DummyDeviceDriver::new(spi_dev);
    loop {
        info!("spi task A");
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn task_b(spi_bus: &'static Spi1Bus, cs: rp::gpio::Output<'static>) {
    let spi_dev = SpiDevice::new(spi_bus, cs);
    let _sensor = DummyDeviceDriver::new(spi_dev);
    loop {
        info!("spi task B");
        Timer::after_secs(1).await;
    }
}

struct DummyDeviceDriver<SpiDev: embedded_hal_async::spi::SpiDevice> {
    _spi_dev: SpiDev,
}

impl<SpiDev: embedded_hal_async::spi::SpiDevice> DummyDeviceDriver<SpiDev> {
    fn new(spi_dev: SpiDev) -> Self {
        Self { _spi_dev: spi_dev }
    }
}
