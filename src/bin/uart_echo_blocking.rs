#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp as rp;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    let uart_driver = {
        let tx = p.PIN_0;
        let rx = p.PIN_1;
        let config = rp::uart::Config::default();
        rp::uart::Uart::new_blocking(p.UART0, tx, rx, config)
    };
    run_session(uart_driver).unwrap();
}

fn run_session(mut uart_driver: rp::uart::Uart<'_, rp::uart::Blocking>) -> Result<(), rp::uart::Error> {
    let mut first = true;
    let mut buf = [0u8; 1];
    loop {
        uart_driver.blocking_read(&mut buf)?;
        if first {
            uart_driver.blocking_write(b"\r\nEcho via Blocking UART\r\n")?;
            first = false;
        }
        uart_driver.blocking_write(&buf)?;
    }
    #[allow(unreachable_code)]
    Ok(()) 
}
