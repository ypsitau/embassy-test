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
    let cdc_acm_driver_1 = {
        static STATE: StaticCell<usb::class::cdc_acm::State> = StaticCell::new();
        let state = STATE.init(usb::class::cdc_acm::State::new());
        let max_packet_size = 64;
        usb::class::cdc_acm::CdcAcmClass::new(&mut usb_builder, state, max_packet_size)
    };
    let cdc_acm_driver_2 = {
        static STATE: StaticCell<usb::class::cdc_acm::State> = StaticCell::new();
        let state = STATE.init(usb::class::cdc_acm::State::new());
        let max_packet_size = 64;
        usb::class::cdc_acm::CdcAcmClass::new(&mut usb_builder, state, max_packet_size)
    };
    let mut usb_device = usb_builder.build();
    let fut_usb = usb_device.run();
    let fut_echo_1 = run_session(cdc_acm_driver_1, "USB CDC ACM 1");
    let fut_echo_2 = run_session(cdc_acm_driver_2, "USB CDC ACM 2");
    embassy_futures::join::join3(fut_usb, fut_echo_1, fut_echo_2).await;
}

type CdcAcmDriver<'d> = usb::class::cdc_acm::CdcAcmClass<'d, rp::usb::Driver<'d, rp::peripherals::USB>>;

async fn run_session(mut cdc_acm_driver: CdcAcmDriver<'_>, name: &str) -> Result<(), usb::driver::EndpointError> {
    let mut text_opening = String::<64>::new();
    write!(text_opening, "\r\nEcho via {}\r\n", name).unwrap();
    let mut first = true;
    let mut buf = [0u8; 64];
    let e = loop {
        cdc_acm_driver.wait_connection().await;
        info!("Connected");
        let e = loop {
            let buf_read = match cdc_acm_driver.read_packet(&mut buf).await {
                Ok(n) => &buf[..n], Err(e) => break e,
            };
            if first {
                if let Err(e) = cdc_acm_driver.write_packet(text_opening.as_bytes()).await { break e; }
                first = false;
            }
            if let Err(e) = cdc_acm_driver.write_packet(buf_read).await { break e; }
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
