#![no_std]
#![no_main]

use defmt::expect;
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
    expect!(uart.blocking_write(b"Echo example\r\n"));
    loop {
        let mut buf = [0u8; 1];
        expect!(uart.blocking_read(&mut buf));
        expect!(uart.blocking_write(&buf));
    }
}
