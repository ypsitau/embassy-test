//! This example shows how you can use raw interrupt handlers alongside embassy.
//! The example also showcases some of the options available for sharing resources/data.
//!
//! In the example, an ADC reading is triggered every time the PWM wraps around.
//! The sample data is sent down a channel, to be processed inside a low priority task.
//! The processed data is then used to adjust the PWM duty cycle, once every second.

#![no_std]
#![no_main]

use core::cell::{Cell, RefCell};

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_rp::interrupt;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Ticker};
use portable_atomic::{AtomicU32, Ordering};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

static ATOMIC_COUNTER: AtomicU32 = AtomicU32::new(0);
static MUTEX_PWM: Mutex<CriticalSectionRawMutex, RefCell<Option<rp::pwm::Pwm>>> = Mutex::new(RefCell::new(None));
static MUTEX_ADC: Mutex<CriticalSectionRawMutex, RefCell<Option<(rp::adc::Adc<rp::adc::Blocking>,
    rp::adc::Channel)>>> = Mutex::new(RefCell::new(None));
static CHANNEL_ADC: Channel<CriticalSectionRawMutex, u16, 2048> = Channel::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let adc = rp::adc::Adc::new_blocking(p.ADC, Default::default());
    let adc_ch = rp::adc::Channel::new_pin(p.PIN_26, rp::gpio::Pull::None);
    MUTEX_ADC.lock(|a| a.borrow_mut().replace((adc, adc_ch)));
    let pwm = rp::pwm::Pwm::new_output_b(p.PWM_SLICE4, p.PIN_25, Default::default());
    MUTEX_PWM.lock(|p| p.borrow_mut().replace(pwm));
    // Enable the interrupt for pwm slice 4
    rp::pac::PWM.inte().modify(|w| w.set_ch4(true));
    unsafe {
        cortex_m::peripheral::NVIC::unmask(rp::interrupt::PWM_IRQ_WRAP);
    }
    // Tasks require their resources to have 'static lifetime
    // No Mutex needed when sharing within the same executor/prio level
    let avg = {
        static STATIC_CELL: StaticCell<Cell<u32>> = StaticCell::new();
        STATIC_CELL.init(Default::default())
    };
    spawner.spawn(task_processing(avg).unwrap());
    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        ticker.next().await;
        let freq = ATOMIC_COUNTER.swap(0, Ordering::Relaxed);
        info!("pwm freq: {:?} Hz", freq);
        info!("adc average: {:?}", avg.get());
        // Update the pwm duty cycle, based on the averaged adc reading
        let mut config = rp::pwm::Config::default();
        config.compare_b = ((avg.get() as f32 / 4095.0) * config.top as f32) as _;
        MUTEX_PWM.lock(|p| p.borrow_mut().as_mut().unwrap().set_config(&config));
    }
}

#[embassy_executor::task]
async fn task_processing(avg: &'static Cell<u32>) {
    let mut buffer: heapless::HistoryBuf<u16, 100> = Default::default();
    loop {
        let value = CHANNEL_ADC.receive().await;
        buffer.write(value);
        let sum: u32 = buffer.iter().map(|value| *value as u32).sum();
        avg.set(sum / buffer.len() as u32);
    }
}

#[interrupt]
fn PWM_IRQ_WRAP() {
    critical_section::with(|cs| {
        let mut adc_pack = MUTEX_ADC.borrow(cs).borrow_mut();
        let (adc, adc_ch) = adc_pack.as_mut().unwrap();
        let value = adc.blocking_read(adc_ch).unwrap();
        CHANNEL_ADC.try_send(value).ok();
        // Clear the interrupt, so we don't immediately re-enter this irq handler
        MUTEX_PWM.borrow(cs).borrow_mut().as_mut().unwrap().clear_wrapped();
    });
    ATOMIC_COUNTER.fetch_add(1, Ordering::Relaxed);
}
