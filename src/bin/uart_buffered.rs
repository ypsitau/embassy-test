#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals;
use embassy_rp::uart;
use embedded_io_async::{Write, Read};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    UART0_IRQ => uart::BufferedInterruptHandler<peripherals::UART0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut uart = {
        let tx = p.PIN_0;
        let rx = p.PIN_1;
        static TX_BUFFER: static_cell::StaticCell<[u8; 64]> = static_cell::StaticCell::new();
        static RX_BUFFER: static_cell::StaticCell<[u8; 64]> = static_cell::StaticCell::new();
        let tx_buffer = TX_BUFFER.init([0u8; 64]);
        let rx_buffer = RX_BUFFER.init([0u8; 64]);
        let config = uart::Config::default();
        uart::BufferedUart::new(p.UART0, tx, rx, Irqs, tx_buffer, rx_buffer, config)
    };
    uart.write_all(b"Echo example\r\n").await.unwrap();
    loop {
        let mut buf = [0u8; 32];
        let bytes_read = uart.read(&mut buf).await.unwrap();
        uart.write_all(&buf[..bytes_read]).await.unwrap();
    }
}
