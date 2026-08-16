//! This example shows how to share (async) I2C and SPI buses between multiple devices.

#![no_std]
#![no_main]

use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, SPI1};
use embassy_rp::spi::{self, Spi};
use embassy_rp::{bind_interrupts, dma};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

type Spi1Bus = Mutex<NoopRawMutex, Spi<'static, SPI1, spi::Async>>;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>, dma::InterruptHandler<DMA_CH1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    info!("Here we go!");

    // Shared SPI bus
    let spi_cfg = spi::Config::default();
    let spi = Spi::new(
        p.SPI1, p.PIN_10, p.PIN_11, p.PIN_12, p.DMA_CH0, p.DMA_CH1, Irqs, spi_cfg,
    );
    static SPI_BUS: StaticCell<Spi1Bus> = StaticCell::new();
    let spi_bus = SPI_BUS.init(Mutex::new(spi));

    // Chip select pins for the SPI devices
    let cs_a = Output::new(p.PIN_0, Level::High);
    let cs_b = Output::new(p.PIN_1, Level::High);

    spawner.spawn(spi_task_a(spi_bus, cs_a).unwrap());
    spawner.spawn(spi_task_b(spi_bus, cs_b).unwrap());
}

#[embassy_executor::task]
async fn spi_task_a(spi_bus: &'static Spi1Bus, cs: Output<'static>) {
    let spi_dev = SpiDevice::new(spi_bus, cs);
    let _sensor = DummySpiDeviceDriver::new(spi_dev);
    loop {
        info!("spi task A");
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn spi_task_b(spi_bus: &'static Spi1Bus, cs: Output<'static>) {
    let spi_dev = SpiDevice::new(spi_bus, cs);
    let _sensor = DummySpiDeviceDriver::new(spi_dev);
    loop {
        info!("spi task B");
        Timer::after_secs(1).await;
    }
}

// Dummy SPI device driver, using `embedded-hal-async`
struct DummySpiDeviceDriver<SPI: embedded_hal_async::spi::SpiDevice> {
    _spi: SPI,
}

impl<SPI: embedded_hal_async::spi::SpiDevice> DummySpiDeviceDriver<SPI> {
    fn new(spi_dev: SPI) -> Self {
        Self { _spi: spi_dev }
    }
}
