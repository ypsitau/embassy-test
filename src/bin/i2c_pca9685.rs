#![no_std]
#![no_main]

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
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    let mutex_i2c1 = {
        let scl = p.PIN_15;
        let sda = p.PIN_14;
        let config = rp::i2c::Config::default();
        // should be replaced by make_static macro when it becomes available
        static STATIC_CELL: StaticCell<MutexI2C1> = StaticCell::new();
        STATIC_CELL.init(MutexI2C1::new(rp::i2c::I2c::new_async(p.I2C1, scl, sda, Irqs, config)))
    };
    // impl embedded_hal_async::i2c::I2c
    let i2c_device = embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice::new(mutex_i2c1);
    let fut_pwm_task = task_pwm(i2c_device);
    fut_pwm_task.await;
}

async fn task_pwm(i2c_device: impl embedded_hal_async::i2c::I2c) {
    use pwm_pca9685::Channel;
    let mut pwm = {
        let address = pwm_pca9685::Address::default();
        pwm_pca9685::Pca9685::new(i2c_device, address).unwrap()
    };
    // This corresponds to a frequency of 60 Hz.
    pwm.set_prescale(100).await.unwrap();
    // It is necessary to enable the device.
    pwm.enable().await.unwrap();
    // Turn on channel 0 at 0x000.
    pwm.set_channel_on(Channel::C0, 0x000).await.unwrap();
    // Turn off channel 0 at 0x7ff, which is 50% in the range `[0x000..=0xfff]`.
    pwm.set_channel_off(Channel::C0, 0x7ff).await.unwrap();
}
