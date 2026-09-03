#![no_std]
#![no_main]

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
        rp::dma::InterruptHandler<rp::peripherals::DMA_CH1>,
        rp::dma::InterruptHandler<rp::peripherals::DMA_CH2>,
        rp::dma::InterruptHandler<rp::peripherals::DMA_CH3>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    let spi0 = {
        let clk = p.PIN_2;
        let mosi = p.PIN_3;
        let miso = p.PIN_4;
        let tx_dma = p.DMA_CH0;
        let rx_dma = p.DMA_CH1;
        let config = rp::spi::Config::default();
        rp::spi::Spi::new(p.SPI0, clk, mosi, miso, tx_dma, rx_dma, Irqs, config)
    };
    let mutex_spi1 = {
        let clk = p.PIN_10;
        let mosi = p.PIN_11;
        let miso = p.PIN_12;
        let tx_dma = p.DMA_CH2;
        let rx_dma = p.DMA_CH3;
        let config = rp::spi::Config::default();
        let spi = rp::spi::Spi::new(p.SPI1, clk, mosi, miso, tx_dma, rx_dma, Irqs, config);
        // should be replaced by make_static macro when it becomes available
        static STATIC_CELL: StaticCell<MutexSPI1> = StaticCell::new();
        STATIC_CELL.init(MutexSPI1::new(spi))
    };
    let fut_task_spi0 = async {
        let gpio_cs = rp::gpio::Output::new(p.PIN_5, rp::gpio::Level::High);
        let spi_device = embedded_hal_bus::spi::ExclusiveDevice::new(spi0, gpio_cs, embassy_time::Delay).unwrap();
        let _sensor = DummyDeviceDriver::new(spi_device);
        loop {
            Timer::after_secs(1).await;
        }
    };
    let fut_task_sp1_a = async {
        let gpio_cs = rp::gpio::Output::new(p.PIN_13, rp::gpio::Level::High);
        let spi_device = embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice::new(mutex_spi1, gpio_cs);
        let _sensor = DummyDeviceDriver::new(spi_device);
        loop {
            Timer::after_secs(1).await;
        }
    };
    let fut_task_sp1_b = async {
        let gpio_cs = rp::gpio::Output::new(p.PIN_14, rp::gpio::Level::High);
        let spi_device = embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice::new(mutex_spi1, gpio_cs);
        let _sensor = DummyDeviceDriver::new(spi_device);
        loop {
            Timer::after_secs(1).await;
        }
    };
    embassy_futures::join::join3(fut_task_spi0, fut_task_sp1_a, fut_task_sp1_b).await;
}

struct DummyDeviceDriver<SpiDevice> {
    _spi_device: SpiDevice,
}

impl<SpiDevice: embedded_hal_async::spi::SpiDevice> DummyDeviceDriver<SpiDevice> {
    fn new(spi_device: SpiDevice) -> Self {
        Self { _spi_device: spi_device }
    }
}
