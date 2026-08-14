#![no_std]
#![no_main]

use defmt::info;
use core::fmt::Write as _;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals;
use embassy_rp::uart;
use embedded_io_async::Write as _;
use embassy_time::Timer;
use heapless::String;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    UART0_IRQ => uart::BufferedInterruptHandler<peripherals::UART0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let uart_driver = {
        let tx = p.PIN_0;
        static TX_BUFFER: static_cell::StaticCell<[u8; 64]> = static_cell::StaticCell::new();
        let tx_buffer = TX_BUFFER.init([0u8; 64]);
        let config = uart::Config::default();
        uart::BufferedUartTx::new(p.UART0, Irqs, tx, tx_buffer, config)
    };
    run_session(uart_driver).await.unwrap();
}

async fn run_session(mut uart_driver: uart::BufferedUartTx) -> Result<(), uart::Error> {
    let mut text = String::<64>::new();
    let mut i = 0;
    loop {
        info!("Sending message {}", i);
        text.clear();
        write!(text, "Hello from Buffered UART TX {}!\r\n", i).unwrap();
        uart_driver.write_all(text.as_bytes()).await?;
        i += 1;
        Timer::after_millis(1000).await;
    }
    #[allow(unreachable_code)]
    Ok(())
}
