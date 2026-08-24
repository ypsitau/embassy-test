#![no_std]
#![no_main]

use core::sync::atomic;
use defmt::info;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_usb as usb;
use embedded_cli as cli;
use embedded_cli::cli::CliBuilder;
use static_cell::StaticCell;
use ufmt::uwriteln;
use {defmt_rtt as _, panic_probe as _};

#[derive(cli::Command)]
enum CommandLine<'a> {
    /// Show a greeting.
    Hello { name: Option<&'a str> },
    /// Show the firmware status.
    Status,
    /// Show GPIO Status.
    GpioStatus,
    
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
    let (writer, fut_writer) = {
        static CHANNEL: channel::Channel<ThreadModeRawMutex, u8, 256> = channel::Channel::new();
        (
            WriterCDCACM::new(CHANNEL.sender()),
            WriterCDCACM::run_channel_task(cdc_acm_sender, CHANNEL.receiver()),
        )
    };
    let fut_cli = async {
        let (command_buffer, history_buffer) = {
            static COMMAND_BUFFER: StaticCell<[u8; 128]> = StaticCell::new();
            static HISTORY_BUFFER: StaticCell<[u8; 32]> = StaticCell::new();
            (*COMMAND_BUFFER.init([0; 128]), *HISTORY_BUFFER.init([0; 32]))
        };
        let mut cli = match CliBuilder::default()
            .writer(writer)
            .command_buffer(command_buffer)
            .history_buffer(history_buffer)
            .build()
        {
            Ok(cli) => cli,
            Err(_) => return,
        };
        let packet_buf = {
            static PACKET_BUF: StaticCell<[u8; 64]> = StaticCell::new();
            PACKET_BUF.init([0u8; 64])
        };
        let e = loop {
            cdc_acm_receiver.wait_connection().await;
            info!("Connected");
            let e = loop {
                let buf_read = match cdc_acm_receiver.read_packet(packet_buf).await {
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
                                CommandLine::GpioStatus => {
                                    uwriteln!(cli.writer(), "GPIO status: ok")?;
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
    embassy_futures::join::join3(fut_usb, fut_writer, fut_cli).await;
}

//-----------------------------------------------------------------------------s
// WriterCDCACM
//-----------------------------------------------------------------------------s
struct WriterCDCACM<'a> {
    sender_to_channel: channel::Sender<'a, ThreadModeRawMutex, u8, 256>,
}

impl<'a> WriterCDCACM<'a> {
    fn new(sender_to_channel: channel::Sender<'a, ThreadModeRawMutex, u8, 256>) -> Self {
        Self { sender_to_channel }
    }
    async fn run_channel_task(
        mut sender_to_usb: usb::class::cdc_acm::Sender<'static, rp::usb::Driver<'static, rp::peripherals::USB>>,
        reciver_from_channel: channel::Receiver<'a, ThreadModeRawMutex, u8, 256>,
    ) -> ! {
        let mut packet = [0u8; 64];
        loop {
            let first = reciver_from_channel.receive().await;
            let mut length = 0;
            packet[length] = first;
            length += 1;
            while length < packet.len() {
                match reciver_from_channel.try_receive() {
                    Ok(byte) => {
                        packet[length] = byte;
                        length += 1;
                    }
                    Err(_) => break,
                }
            }
            let _ = sender_to_usb.write_packet(&packet[..length]).await;
        }
    }
}

impl embedded_io::ErrorType for WriterCDCACM<'_> {
    type Error = WriterError;
}

impl embedded_io::Write for WriterCDCACM<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        for &byte in buf {
            self.sender_to_channel.try_send(byte).map_err(|_| WriterError)?;
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
