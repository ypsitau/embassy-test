#![no_std]
#![no_main]

use defmt::info;
use core::fmt::Write as _;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embedded_io_async::Write as _;
use embassy_time::Timer;
use heapless::String;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

rp::bind_interrupts!(struct Irqs {
    UART0_IRQ => rp::uart::BufferedInterruptHandler<rp::peripherals::UART0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    let uart_driver = {
        let tx = p.PIN_0;
        let tx_buffer = { // should be replaced by make_static macro when it becomes available
            const TX_BUFFER_SIZE: usize = 64;
            static STATIC_CELL: StaticCell<[u8; TX_BUFFER_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0u8; TX_BUFFER_SIZE])
        };
        let config = rp::uart::Config::default();
        rp::uart::BufferedUartTx::new(p.UART0, Irqs, tx, tx_buffer, config)
    };
    run_session(uart_driver).await.unwrap();
}

async fn run_session(mut uart_driver: rp::uart::BufferedUartTx) -> Result<(), rp::uart::Error> {
    Timer::after_millis(1000).await;
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
