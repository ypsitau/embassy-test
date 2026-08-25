#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp as rp;
use heapless::String;
use core::fmt::Write as _;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    let uart_driver = {
        let tx = p.PIN_0;
        let config = rp::uart::Config::default();
        rp::uart::UartTx::new_blocking(p.UART0, tx, config)
    };
    run_session(uart_driver).await.unwrap();
}

async fn run_session(mut uart_driver: rp::uart::UartTx<'_, rp::uart::Blocking>) -> Result<(), rp::uart::Error> {
    let mut text = String::<64>::new();
    let mut i = 0;
    loop {
        text.clear();
        write!(text, "Hello from Blocking UART TX {}!\r\n", i).unwrap();
        uart_driver.blocking_write(text.as_bytes())?;
        i += 1;
        Timer::after_millis(1000).await;
    }
    #[allow(unreachable_code)]
    Ok(()) 
}
