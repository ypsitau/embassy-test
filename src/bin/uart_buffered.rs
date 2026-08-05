#![no_std]
#![no_main]

use defmt::expect;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals;
use embassy_rp::uart;
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    UART0_IRQ => uart::BufferedInterruptHandler<peripherals::UART0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut tx_buffer = [0u8; 64];
    let mut rx_buffer = [0u8; 64];
    let mut uart = {
        let tx = p.PIN_0;
        let rx = p.PIN_1;
        let config = uart::Config::default();
        uart::BufferedUart::new(p.UART0, tx, rx, Irqs, &mut tx_buffer, &mut rx_buffer, config)
    };
    expect!(uart.write_all(b"Hello World!\r\n").await);
    loop {
        expect!(uart.write_all(b"hello there!\r\n").await);
        Timer::after(Duration::from_secs(1)).await;
    }
}
