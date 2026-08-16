//! This example shows how to share (async) I2C and SPI buses between multiple devices.

#![no_std]
#![no_main]

use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_rp as rp;
//use embassy_rp::i2c::{self, I2c, InterruptHandler};
//use embassy_rp::peripherals::{I2C1};
//use embassy_rp::{bind_interrupts};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

type I2c1Bus = Mutex<NoopRawMutex, rp::i2c::I2c<'static, rp::peripherals::I2C1, rp::i2c::Async>>;

rp::bind_interrupts!(struct Irqs {
    I2C1_IRQ => rp::i2c::InterruptHandler<rp::peripherals::I2C1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    info!("Here we go!");

    // Shared I2C bus
    let i2c = rp::i2c::I2c::new_async(p.I2C1, p.PIN_15, p.PIN_14, Irqs, rp::i2c::Config::default());
    static I2C_BUS: StaticCell<I2c1Bus> = StaticCell::new();
    let i2c_bus = I2C_BUS.init(Mutex::new(i2c));

    spawner.spawn(i2c_task_a(i2c_bus).unwrap());
    spawner.spawn(i2c_task_b(i2c_bus).unwrap());

}

#[embassy_executor::task]
async fn i2c_task_a(i2c_bus: &'static I2c1Bus) {
    let i2c_dev = I2cDevice::new(i2c_bus);
    let _sensor = DummyI2cDeviceDriver::new(i2c_dev, 0xC0);
    loop {
        info!("i2c task A");
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn i2c_task_b(i2c_bus: &'static I2c1Bus) {
    let i2c_dev = I2cDevice::new(i2c_bus);
    let _sensor = DummyI2cDeviceDriver::new(i2c_dev, 0xDE);
    loop {
        info!("i2c task B");
        Timer::after_secs(1).await;
    }
}

// Dummy I2C device driver, using `embedded-hal-async`
struct DummyI2cDeviceDriver<I2C: embedded_hal_async::i2c::I2c> {
    _i2c: I2C,
}

impl<I2C: embedded_hal_async::i2c::I2c> DummyI2cDeviceDriver<I2C> {
    fn new(i2c_dev: I2C, _address: u8) -> Self {
        Self { _i2c: i2c_dev }
    }
}
