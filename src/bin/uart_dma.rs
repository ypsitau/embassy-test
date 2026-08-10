#![no_std]
#![no_main]

use defmt::expect;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::dma;
use embassy_rp::peripherals;
use embassy_rp::uart;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    UART0_IRQ => uart::InterruptHandler<peripherals::UART0>;
    DMA_IRQ_0 => dma::InterruptHandler<peripherals::DMA_CH0>, dma::InterruptHandler<peripherals::DMA_CH1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut uart = {
        let tx = p.PIN_0;
        let rx = p.PIN_1;
        let tx_dma = p.DMA_CH0;
        let rx_dma = p.DMA_CH1;
        let config = uart::Config::default();
        uart::Uart::new(p.UART0, tx, rx, Irqs, tx_dma, rx_dma, config)
    };
    expect!(uart.write(b"Echo example\r\n").await);
    loop {
        let mut buf = [0u8; 1];
        expect!(uart.read(&mut buf).await);
        expect!(uart.write(&buf).await);
    }
}
