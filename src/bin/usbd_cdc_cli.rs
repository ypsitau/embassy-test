#![no_std]
#![no_main]

use core::sync::atomic;
use defmt::info;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_usb as usb;
use embedded_cli::cli::CliBuilder;
use static_cell::StaticCell;
//use ufmt::uwriteln;
use {defmt_rtt as _, panic_probe as _};

#[derive(embedded_cli::Command)]
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
        const VID: u16 = 0xc0de;
        const PID: u16 = 0xcafe;
        const CONFIG_DESCRIPTOR_SIZE: usize = 256;
        const BOS_DESCRIPTOR_SIZE: usize = 256;
        const MSOS_DESCRIPTOR_SIZE: usize = 256;
        const CONTROL_BUF_SIZE: usize = 64;
        let mut config = usb::Config::new(VID, PID);
        config.manufacturer = Some("Embassy");
        config.product = Some("usbd_cdc_cli");
        config.serial_number = Some("12345678");
        config.max_power = 100;
        config.max_packet_size_0 = CONTROL_BUF_SIZE as u8;
         let config_descriptor_buf = { // should be replaced by make_static macro when it becomes available
            static STATIC_CELL: StaticCell<[u8; CONFIG_DESCRIPTOR_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0; CONFIG_DESCRIPTOR_SIZE])
        };
        let bos_descriptor_buf = { // should be replaced by make_static macro when it becomes available
            static STATIC_CELL: StaticCell<[u8; BOS_DESCRIPTOR_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0; BOS_DESCRIPTOR_SIZE])
        };
        let msos_descriptor_buf = { // should be replaced by make_static macro when it becomes available
            static STATIC_CELL: StaticCell<[u8; MSOS_DESCRIPTOR_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0; MSOS_DESCRIPTOR_SIZE])
        };
        let control_buf = { // should be replaced by make_static macro when it becomes available
            static STATIC_CELL: StaticCell<[u8; CONTROL_BUF_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0; CONTROL_BUF_SIZE])
        };
        let device_handler = { // should be replaced by make_static macro when it becomes available
            static STATIC_CELL: StaticCell<DeviceHandler> = StaticCell::new();
            STATIC_CELL.init(DeviceHandler::new())
        };
        let mut usb_builder = usb::Builder::new(usb_driver, config,
            config_descriptor_buf, bos_descriptor_buf, msos_descriptor_buf, control_buf);
        usb_builder.handler(device_handler);
        usb_builder
    };
    let (cdc_sender, mut cdc_receiver) = {
        static STATE: StaticCell<usb::class::cdc_acm::State> = StaticCell::new();
        let state = STATE.init(usb::class::cdc_acm::State::new());
        let max_packet_size = 64;
        usb::class::cdc_acm::CdcAcmClass::new(&mut usb_builder, state, max_packet_size).split()
    };
    let mut usb_device = usb_builder.build();
    let fut_usb = usb_device.run();
    let (writer_async_bridge, fut_writer_async_bridge) = {
        const N: usize = 256;
        static CHANNEL: channel::Channel<ThreadModeRawMutex, u8, N> = channel::Channel::new();
        (WriterAsyncBridge::<N>::new(CHANNEL.sender()), WriterAsyncBridge::<N>::run_task(cdc_sender, CHANNEL.receiver()))
    };
    let fut_cli = async {
        let command_buffer = { // should be replaced by make_static macro when it becomes available
            const COMMAND_BUFFER_SIZE: usize = 128;
            static STATIC_CELL: StaticCell<[u8; COMMAND_BUFFER_SIZE]> = StaticCell::new();
            *STATIC_CELL.init([0; COMMAND_BUFFER_SIZE])
        };
        let history_buffer = { // should be replaced by make_static macro when it becomes available
            const HISTORY_BUFFER_SIZE: usize = 512;
            static STATIC_CELL: StaticCell<[u8; HISTORY_BUFFER_SIZE]> = StaticCell::new();
            *STATIC_CELL.init([0; HISTORY_BUFFER_SIZE])
        };
        let mut cli = CliBuilder::default()
            .writer(writer_async_bridge)
            .command_buffer(command_buffer)
            .history_buffer(history_buffer)
            .build().expect("Failed to build CLI");
        let packet_buf = { // should be replaced by make_static macro when it becomes available
            const PACKET_BUF_SIZE: usize = 64;
            static STATIC_CELL: StaticCell<[u8; PACKET_BUF_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0u8; PACKET_BUF_SIZE])
        };
        let e = loop {
            cdc_receiver.wait_connection().await;
            info!("Connected");
            let e = loop {
                let buf_read = match cdc_receiver.read_packet(packet_buf).await {
                    Ok(n) => &packet_buf[..n], Err(e) => break e,
                };
                for &byte in buf_read {
                    let _ = cli.process_byte::<CommandLine, _>(
                        byte,
                        &mut CommandLine::processor(|cli, command| {
                            match command {
                                CommandLine::Hello { name } => {
                                    ufmt::uwriteln!(cli.writer(), "Hello, {}!", name.unwrap_or("world"))?;
                                }
                                CommandLine::Status => {
                                    ufmt::uwriteln!(cli.writer(), "status: ok")?;
                                }
                                CommandLine::GpioStatus => {
                                    ufmt::uwriteln!(cli.writer(), "GPIO status: ok")?;
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
    embassy_futures::join::join3(fut_usb, fut_writer_async_bridge, fut_cli).await;
}

//-----------------------------------------------------------------------------s
// WriterAsyncBridge
//-----------------------------------------------------------------------------s
struct WriterAsyncBridge<'a, const N: usize> {
    sender_to_channel: channel::Sender<'a, ThreadModeRawMutex, u8, N>,
}

impl<'a, const N: usize> WriterAsyncBridge<'a, N> {
    fn new(sender_to_channel: channel::Sender<'a, ThreadModeRawMutex, u8, N>) -> Self {
        Self { sender_to_channel }
    }
    async fn run_task(
        mut sender_to_async: impl embedded_io_async::Write,
        reciver_from_channel: channel::Receiver<'a, ThreadModeRawMutex, u8, N>,
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
            let _ = sender_to_async.write(&packet[..length]).await;
        }
    }
}

impl<'a, const N: usize> embedded_io::ErrorType for WriterAsyncBridge<'a, N> {
    type Error = WriterError;
}

impl<'a, const N: usize> embedded_io::Write for WriterAsyncBridge<'a, N> {
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
