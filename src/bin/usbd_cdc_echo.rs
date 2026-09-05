#![no_std]
#![no_main]

use core::fmt::Write;
use core::sync::atomic;
use defmt::info;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_usb as usb;
use heapless::String;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

//type CdcDriver<'d> = usb::class::cdc_acm::CdcAcmClass<'d, rp::usb::Driver<'d, rp::peripherals::USB>>;
type CdcSender<'d> = usb::class::cdc_acm::Sender<'d, rp::usb::Driver<'d, rp::peripherals::USB>>;
type CdcReceiver<'d> = usb::class::cdc_acm::Receiver<'d, rp::usb::Driver<'d, rp::peripherals::USB>>;

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
        let mut usb_config = usb::Config::new(VID, PID);
        usb_config.manufacturer = Some("Embassy");
        usb_config.product = Some("usbd_cdc_echo");
        usb_config.serial_number = Some("12345678");
        usb_config.max_power = 100;
        usb_config.max_packet_size_0 = CONTROL_BUF_SIZE as u8;
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
        let mut usb_builder = usb::Builder::new(usb_driver, usb_config,
            config_descriptor_buf, bos_descriptor_buf, msos_descriptor_buf, control_buf);
        usb_builder.handler(device_handler);
        usb_builder
    };
    let cdc_driver_1 = {
        let state = {
            static STATE: StaticCell<usb::class::cdc_acm::State> = StaticCell::new();
            STATE.init(usb::class::cdc_acm::State::new())
        };
        let max_packet_size = 64;
        usb::class::cdc_acm::CdcAcmClass::new(&mut usb_builder, state, max_packet_size)
    };
    let cdc_driver_2 = {
        let state = {
            static STATE: StaticCell<usb::class::cdc_acm::State> = StaticCell::new();
            STATE.init(usb::class::cdc_acm::State::new())
        };
        let max_packet_size = 64;
        usb::class::cdc_acm::CdcAcmClass::new(&mut usb_builder, state, max_packet_size)
    };
    let mut usb_device = usb_builder.build();
    let fut_usb = usb_device.run();
    let fut_echo_1 = {
        let (cdc_sender, cdc_receiver) = cdc_driver_1.split();
        run_session(cdc_sender, cdc_receiver, "USB CDC ACM 1")
    };
    let fut_echo_2 = {
        let (cdc_sender, cdc_receiver) = cdc_driver_2.split();
        run_session(cdc_sender, cdc_receiver, "USB CDC ACM 2")
    };
    embassy_futures::join::join3(fut_usb, fut_echo_1, fut_echo_2).await;
}

async fn run_session(mut cdc_sender: CdcSender<'_>, mut cdc_receiver: CdcReceiver<'_>, name: &str) -> Result<(), usb::driver::EndpointError> {
    let mut text_opening = String::<64>::new();
    write!(text_opening, "\r\nEcho via {}\r\n", name).unwrap();
    let mut first = true;
    let mut buf = [0u8; 64];
    let e = loop {
        cdc_receiver.wait_connection().await;
        info!("Connected");
        let e = loop {
            let buf_read = match cdc_receiver.read_packet(&mut buf).await {
                Ok(n) => &buf[..n], Err(e) => break e,
            };
            if first {
                if let Err(e) = cdc_sender.write_packet(text_opening.as_bytes()).await { break e; }
                first = false;
            }
            if let Err(e) = cdc_sender.write_packet(buf_read).await { break e; }
        };
        if e != usb::driver::EndpointError::Disabled { break e; }
    };
    Err(e)
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
