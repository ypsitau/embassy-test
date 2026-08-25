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
        let (tx_buffer, rx_buffer) = {
            const TX_BUFFER_SIZE: usize = 64;
            const RX_BUFFER_SIZE: usize = 64;
            static TX_BUFFER: static_cell::StaticCell<[u8; TX_BUFFER_SIZE]> = static_cell::StaticCell::new();
            static RX_BUFFER: static_cell::StaticCell<[u8; RX_BUFFER_SIZE]> = static_cell::StaticCell::new();
            (TX_BUFFER.init([0u8; TX_BUFFER_SIZE]), RX_BUFFER.init([0u8; RX_BUFFER_SIZE]))
        };
        let config = rp::uart::Config::default();
        rp::uart::BufferedUart::new(p.UART0, tx, rx, Irqs, tx_buffer, rx_buffer, config)
    };
    let (uart_tx, uart_rx) = uart_driver.split();
    run_session(uart_tx, uart_rx).await.unwrap();
}

async fn run_session(mut uart_tx: rp::uart::BufferedUartTx, mut uart_rx: rp::uart::BufferedUartRx) -> Result<(), rp::uart::Error> {
    let mut first = true;
    let mut buf = [0u8; 64];
    loop {
        let n = uart_rx.read(&mut buf).await?;
        if first {
            uart_tx.write_all(b"\r\nEcho via Buffered UART\r\n").await?;
            first = false;
        }
        uart_tx.write_all(&buf[..n]).await?;
    }
    #[allow(unreachable_code)]
    Ok(()) 
}
