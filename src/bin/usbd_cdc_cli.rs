#![no_std]
#![no_main]

use core::sync::atomic;
use defmt::info;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_usb as usb;
use embedded_cli::{cli::CliBuilder, Command};
use static_cell::StaticCell;
use ufmt::uwriteln;
use {defmt_rtt as _, panic_probe as _};

static OUTPUT: Channel<ThreadModeRawMutex, u8, 256> = Channel::new();

#[derive(Command)]
enum CommandLine<'a> {
    /// Show a greeting.
    Hello { name: Option<&'a str> },
    /// Show the firmware status.
    Status,
}

rp::bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => rp::usb::InterruptHandler<rp::peripherals::USB>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    let usb_driver = rp::usb::Driver::new(p.USB, Irqs);
    let mut usb_builder = {
        let mut config = usb::Config::new(0xc0de, 0xcafe);
        config.manufacturer = Some("Embassy");
        config.product = Some("USB-serial example");
        config.serial_number = Some("12345678");
        config.max_power = 100;
        config.max_packet_size_0 = 64;
        static CONFIG_DESCRIPTOR_BUF: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR_BUF: StaticCell<[u8; 256]> = StaticCell::new();
        static MSOS_DESCRIPTOR_BUF: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF_BUF: StaticCell<[u8; 64]> = StaticCell::new();
        let mut usb_builder = usb::Builder::new(
            usb_driver,
            config,
            CONFIG_DESCRIPTOR_BUF.init([0; 256]),
            BOS_DESCRIPTOR_BUF.init([0; 256]),
            MSOS_DESCRIPTOR_BUF.init([0; 256]),
            CONTROL_BUF_BUF.init([0; 64]),
        );
        static DEVICE_HANDLER: StaticCell<DeviceHandler> = StaticCell::new();
        usb_builder.handler(DEVICE_HANDLER.init(DeviceHandler::new()));
        usb_builder
    };
    let (cdc_acm_sender, mut cdc_acm_receiver) = {
        static STATE: StaticCell<usb::class::cdc_acm::State> = StaticCell::new();
        let state = STATE.init(usb::class::cdc_acm::State::new());
        let max_packet_size = 64;
        usb::class::cdc_acm::CdcAcmClass::new(&mut usb_builder, state, max_packet_size).split()
    };
    let mut usb_device = usb_builder.build();
    let fut_usb = usb_device.run();
    let fut_output = output_task(cdc_acm_sender, OUTPUT.receiver());
    let fut_cli = async {
        let writer = WriterCDCACM { output: OUTPUT.sender() };
        static COMMAND_BUFFER: StaticCell<[u8; 128]> = StaticCell::new();
        static HISTORY_BUFFER: StaticCell<[u8; 32]> = StaticCell::new();
        let mut cli = match CliBuilder::default()
            .writer(writer)
            .command_buffer(*COMMAND_BUFFER.init([0; 128]))
            .history_buffer(*HISTORY_BUFFER.init([0; 32]))
            .build()
        {
            Ok(cli) => cli,
            Err(_) => return,
        };
        let mut packet_buf = [0u8; 64];
        let e = loop {
            cdc_acm_receiver.wait_connection().await;
            info!("Connected");
            let e = loop {
                let buf_read = match cdc_acm_receiver.read_packet(&mut packet_buf).await {
                    Ok(n) => &packet_buf[..n], Err(e) => break e,
                };
                for &byte in buf_read {
                    let _ = cli.process_byte::<CommandLine, _>(
                        byte,
                        &mut CommandLine::processor(|cli, command| {
                            match command {
                                CommandLine::Hello { name } => {
                                    uwriteln!(cli.writer(), "Hello, {}!", name.unwrap_or("world"))?;
                                }
                                CommandLine::Status => {
                                    uwriteln!(cli.writer(), "status: ok")?;
                                }
                            }
                            Ok(())
                        }),
                    );
                }
            };
            if e != usb::driver::EndpointError::Disabled { break e; }
        };
        panic!("USB error: {:?}", e);
    };
    embassy_futures::join::join3(fut_usb, fut_output, fut_cli).await;
}

async fn output_task(
    mut sender: usb::class::cdc_acm::Sender<'static, rp::usb::Driver<'static, rp::peripherals::USB>>,
    receiver: Receiver<'static, ThreadModeRawMutex, u8, 256>,
) -> ! {
    let mut packet = [0u8; 64];
    loop {
        let first = receiver.receive().await;
        let mut length = 0;
        packet[length] = first;
        length += 1;
        while length < packet.len() {
            match receiver.try_receive() {
                Ok(byte) => {
                    packet[length] = byte;
                    length += 1;
                }
                Err(_) => break,
            }
        }
        let _ = sender.write_packet(&packet[..length]).await;
    }
}

//-----------------------------------------------------------------------------s
// WriterCDCACM
//-----------------------------------------------------------------------------s
struct WriterCDCACM<'d> {
    output: Sender<'d, ThreadModeRawMutex, u8, 256>,
}

impl embedded_io::ErrorType for WriterCDCACM<'_> {
    type Error = WriterError;
}

impl embedded_io::Write for WriterCDCACM<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        for &byte in buf {
            self.output.try_send(byte).map_err(|_| WriterError)?;
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
struct WriterError;

impl embedded_io::Error for WriterError {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}

//-----------------------------------------------------------------------------
// DeviceHandler
//-----------------------------------------------------------------------------
struct DeviceHandler {
    configured: atomic::AtomicBool,
}

impl DeviceHandler {
    fn new() -> Self {
        DeviceHandler {
            configured: atomic::AtomicBool::new(false),
        }
    }
}

impl usb::Handler for DeviceHandler {
    /// Called when the USB device has been enabled or disabled.
    fn enabled(&mut self, enabled: bool) {
        info!("usb::Handler.enabled({})", enabled);
        self.configured.store(false, atomic::Ordering::Relaxed);
    }
    /// Called after a USB reset after the bus reset sequence is complete.
    fn reset(&mut self) {
        info!("usb::Handler.reset()");
        self.configured.store(false, atomic::Ordering::Relaxed);
    }
    /// Called when the host has set the address of the device to `addr`.
    fn addressed(&mut self, addr: u8) {
        info!("usb::Handler.addressed(addr: {})", addr);
        self.configured.store(false, atomic::Ordering::Relaxed);
    }
    /// Called when the host has enabled or disabled the configuration of the device.
    fn configured(&mut self, configured: bool) {
        info!("usb::Handler.configured(configured: {})", configured);
        self.configured.store(configured, atomic::Ordering::Relaxed);
    }
}
