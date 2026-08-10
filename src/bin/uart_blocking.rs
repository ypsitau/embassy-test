#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::uart;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut uart = {
        let tx = p.PIN_0;
        let rx = p.PIN_1;
        let config = uart::Config::default();
        uart::Uart::new_blocking(p.UART0, tx, rx, config)
    };
    uart.blocking_write(b"Echo example\r\n").unwrap();
    loop {
        let mut buf = [0u8; 1];
        uart.blocking_read(&mut buf).unwrap();
        uart.blocking_write(&buf).unwrap();
    }
}
