#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp as rp;
use {defmt_rtt as _, panic_probe as _};

rp::bind_interrupts!(struct Irqs {
    UART0_IRQ => rp::uart::InterruptHandler<rp::peripherals::UART0>;
    DMA_IRQ_0 => rp::dma::InterruptHandler<rp::peripherals::DMA_CH0>, rp::dma::InterruptHandler<rp::peripherals::DMA_CH1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    let uart_driver = {
        let tx = p.PIN_0;
        let rx = p.PIN_1;
        let tx_dma = p.DMA_CH0;
        let rx_dma = p.DMA_CH1;
        let config = rp::uart::Config::default();
        rp::uart::Uart::new(p.UART0, tx, rx, Irqs, tx_dma, rx_dma, config)
    };
    let (uart_tx, uart_rx) = uart_driver.split();
    run_session(uart_tx, uart_rx).await.unwrap();
}

async fn run_session(mut uart_tx: rp::uart::UartTx<'_, rp::uart::Async>, mut uart_rx: rp::uart::UartRx<'_, rp::uart::Async>) -> Result<(), rp::uart::Error> {
    let mut first = true;
    let mut buf = [0u8; 1];
    loop {
        uart_rx.read(&mut buf).await?;
        if first {
            uart_tx.write(b"\r\nEcho via Async UART\r\n").await?;
            first = false;
        }
        uart_tx.write(&buf).await?;
    }
    #[allow(unreachable_code)]
    Ok(()) 
}
