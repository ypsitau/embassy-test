#![no_std]
#![no_main]

use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use static_cell::StaticCell;
use ssd1306::prelude::*;
use {defmt_rtt as _, panic_probe as _};

type I2c1Bus = Mutex<NoopRawMutex, rp::i2c::I2c<'static, rp::peripherals::I2C1, rp::i2c::Async>>;

rp::bind_interrupts!(struct Irqs {
    I2C1_IRQ => rp::i2c::InterruptHandler<rp::peripherals::I2C1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = rp::init(Default::default());
    let i2c_bus = {
        let config = rp::i2c::Config::default();
        let i2c = rp::i2c::I2c::new_async(p.I2C1, p.PIN_15, p.PIN_14, Irqs, config);
        static I2C_BUS: StaticCell<I2c1Bus> = StaticCell::new();
        I2C_BUS.init(Mutex::new(i2c))
    };
    spawner.spawn(task_a(i2c_bus).unwrap());
    spawner.spawn(task_b(i2c_bus).unwrap());
    spawner.spawn(task_c(i2c_bus).unwrap());
}

#[embassy_executor::task]
async fn task_a(i2c_bus: &'static I2c1Bus) {
    let _sensor = DummyI2cDeviceDriver::new(I2cDevice::new(i2c_bus), 0xC0);
    loop {
        info!("i2c task A");
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn task_b(i2c_bus: &'static I2c1Bus) {
    let _sensor = DummyI2cDeviceDriver::new(I2cDevice::new(i2c_bus), 0xDE);
    loop {
        info!("i2c task B");
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn task_c(i2c_bus: &'static I2c1Bus) {
    let mut display = {
        let interface = ssd1306::I2CDisplayInterface::new(I2cDevice::new(i2c_bus));
        ssd1306::Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode()
    };
    display.init().await.unwrap();
    
}

struct DummyI2cDeviceDriver<I2cDev: embedded_hal_async::i2c::I2c> {
    _i2c_dev: I2cDev,
}
    
impl<I2cDev: embedded_hal_async::i2c::I2c> DummyI2cDeviceDriver<I2cDev> {
    fn new(i2c_dev: I2cDev, _address: u8) -> Self {
        Self { _i2c_dev: i2c_dev }
    }
}
