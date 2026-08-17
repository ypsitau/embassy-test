#![no_std]
#![no_main]

use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice; // impl embedded_hal::spi::SpiDevice
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_time::Timer;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

type MutexSPI1 = embassy_sync::mutex::Mutex<
    embassy_sync::blocking_mutex::raw::NoopRawMutex,
    rp::spi::Spi<'static, rp::peripherals::SPI1, rp::spi::Async>>;

rp::bind_interrupts!(struct Irqs {
    DMA_IRQ_0 =>
        rp::dma::InterruptHandler<rp::peripherals::DMA_CH0>,
        rp::dma::InterruptHandler<rp::peripherals::DMA_CH1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = rp::init(Default::default());
    let mutex_spi1 = {
        let clk = p.PIN_10;
        let mosi = p.PIN_11;
        let miso = p.PIN_12;
        let tx_dma = p.DMA_CH0;
        let rx_dma = p.DMA_CH1;
        let config = rp::spi::Config::default();
        static MUTEX_SPI1: StaticCell<MutexSPI1> = StaticCell::new();
        MUTEX_SPI1.init(MutexSPI1::new(rp::spi::Spi::new(p.SPI1, clk, mosi, miso, tx_dma, rx_dma, Irqs, config)))
    };
    spawner.spawn(task_a(mutex_spi1, rp::gpio::Output::new(p.PIN_0, rp::gpio::Level::High)).unwrap());
    spawner.spawn(task_b(mutex_spi1, rp::gpio::Output::new(p.PIN_1, rp::gpio::Level::High)).unwrap());
}

#[embassy_executor::task]
async fn task_a(mutex_spi1: &'static MutexSPI1, cs: rp::gpio::Output<'static>) {
    let _sensor = DummyDeviceDriver::new(SpiDevice::new(mutex_spi1, cs));
    loop {
        info!("spi task A");
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn task_b(mutex_spi1: &'static MutexSPI1, cs: rp::gpio::Output<'static>) {
    let _sensor = DummyDeviceDriver::new(SpiDevice::new(mutex_spi1, cs));
    loop {
        info!("spi task B");
        Timer::after_secs(1).await;
    }
}

struct DummyDeviceDriver<SpiDevice> {
    _spi_dev: SpiDevice,
}

impl<SpiDevice: embedded_hal_async::spi::SpiDevice> DummyDeviceDriver<SpiDevice> {
    fn new(spi_dev: SpiDevice) -> Self {
        Self { _spi_dev: spi_dev }
    }
}
