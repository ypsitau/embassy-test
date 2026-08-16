#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp as rp;
use embedded_io_async::{Write, Read};
use {defmt_rtt as _, panic_probe as _};

rp::bind_interrupts!(struct Irqs {
    UART0_IRQ => rp::uart::BufferedInterruptHandler<rp::peripherals::UART0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    let uart_driver = {
        let tx = p.PIN_0;
        let rx = p.PIN_1;
        static TX_BUFFER: static_cell::StaticCell<[u8; 64]> = static_cell::StaticCell::new();
        static RX_BUFFER: static_cell::StaticCell<[u8; 64]> = static_cell::StaticCell::new();
        let tx_buffer = TX_BUFFER.init([0u8; 64]);
        let rx_buffer = RX_BUFFER.init([0u8; 64]);
        let config = rp::uart::Config::default();
        rp::uart::BufferedUart::new(p.UART0, tx, rx, Irqs, tx_buffer, rx_buffer, config)
    };
    run_session(uart_driver).await.unwrap();
}

async fn run_session(mut uart_driver: rp::uart::BufferedUart) -> Result<(), rp::uart::Error> {
    let mut first = true;
    let mut buf = [0u8; 64];
    loop {
        let n = uart_driver.read(&mut buf).await?;
        if first {
            uart_driver.write_all(b"\r\nEcho via Buffered UART\r\n").await?;
            first = false;
        }
        uart_driver.write_all(&buf[..n]).await?;
    }
    #[allow(unreachable_code)]
    Ok(()) 
}
