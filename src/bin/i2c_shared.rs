#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_time::Timer;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

type MutexI2C1 = embassy_sync::mutex::Mutex<
    embassy_sync::blocking_mutex::raw::NoopRawMutex,
    rp::i2c::I2c<'static, rp::peripherals::I2C1, rp::i2c::Async>>;

rp::bind_interrupts!(struct Irqs {
    I2C1_IRQ => rp::i2c::InterruptHandler<rp::peripherals::I2C1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = rp::init(Default::default());
    let mutex_i2c1 = {
        let scl = p.PIN_15;
        let sda = p.PIN_14;
        let config = rp::i2c::Config::default();
        // should be replaced by make_static macro when it becomes available
        static STATIC_CELL: StaticCell<MutexI2C1> = StaticCell::new();
        STATIC_CELL.init(MutexI2C1::new(rp::i2c::I2c::new_async(p.I2C1, scl, sda, Irqs, config)))
    };
    spawner.spawn(task_a(mutex_i2c1).unwrap());
    spawner.spawn(task_b(mutex_i2c1).unwrap());
}

#[embassy_executor::task]
async fn task_a(mutex_i2c1: &'static MutexI2C1) {
    // impl embedded_hal_async::i2c::I2c
    let i2c_device = embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice::new(mutex_i2c1);
    let _sensor = DummyDeviceDriver::new(i2c_device, 0xc0);
    loop {
        info!("i2c task A");
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn task_b(mutex_i2c1: &'static MutexI2C1) {
    // impl embedded_hal_async::i2c::I2c
    let i2c_device = embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice::new(mutex_i2c1);
    let _sensor = DummyDeviceDriver::new(i2c_device, 0xde);
    loop {
        info!("i2c task B");
        Timer::after_secs(1).await;
    }
}

struct DummyDeviceDriver<I2c> {
    _i2c: I2c,
}
    
impl<I2c: embedded_hal_async::i2c::I2c> DummyDeviceDriver<I2c> {
    fn new(i2c: I2c, _address: u8) -> Self {
        Self { _i2c: i2c }
    }
}
