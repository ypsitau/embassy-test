#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::uart;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let uart_driver = {
        let tx = p.PIN_0;
        let rx = p.PIN_1;
        let config = uart::Config::default();
        uart::Uart::new_blocking(p.UART0, tx, rx, config)
    };
    do_session(uart_driver).unwrap();
}

fn do_session(mut uart_driver: uart::Uart<'_, uart::Blocking>) -> Result<(), uart::Error> {
    loop {
        let mut buf = [0u8; 1];
        uart_driver.blocking_read(&mut buf)?;
        uart_driver.blocking_write(&buf)?;
    }
    #[allow(unreachable_code)]
    Ok(()) 
}
