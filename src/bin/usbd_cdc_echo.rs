#![no_std]
#![no_main]

use core::fmt::Write;
use core::sync::atomic;
use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals;
use embassy_rp::usb;
use embassy_usb::class as usb_class;
use embassy_usb::driver as usb_driver;
use heapless::String;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => usb::InterruptHandler<peripherals::USB>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let usb_driver = usb::Driver::new(p.USB, Irqs);
    let mut usb_builder = {
        let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
        config.manufacturer = Some("Embassy");
        config.product = Some("USB-serial example");
        config.serial_number = Some("12345678");
        config.max_power = 100;
        config.max_packet_size_0 = 64;
        static CONFIG_DESCRIPTOR_BUF: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR_BUF: StaticCell<[u8; 256]> = StaticCell::new();
        static MSOS_DESCRIPTOR_BUF: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF_BUF: StaticCell<[u8; 64]> = StaticCell::new();
        let mut usb_builder = embassy_usb::Builder::new(
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
    let cdc_acm_1 = {
        static STATE: StaticCell<usb_class::cdc_acm::State> = StaticCell::new();
        let state = STATE.init(usb_class::cdc_acm::State::new());
        let max_packet_size = 64;
        usb_class::cdc_acm::CdcAcmClass::new(&mut usb_builder, state, max_packet_size)
    };
    let cdc_acm_2 = {
        static STATE: StaticCell<usb_class::cdc_acm::State> = StaticCell::new();
        let state = STATE.init(usb_class::cdc_acm::State::new());
        let max_packet_size = 64;
        usb_class::cdc_acm::CdcAcmClass::new(&mut usb_builder, state, max_packet_size)
    };
    let mut usb_device = usb_builder.build();
    let fut_usb = usb_device.run();
    let fut_echo_1 = run_session(cdc_acm_1, "USB CDC ACM 1");
    let fut_echo_2 = run_session(cdc_acm_2, "USB CDC ACM 2");
    embassy_futures::join::join3(fut_usb, fut_echo_1, fut_echo_2).await;
}

type CdcAcm<'d> = usb_class::cdc_acm::CdcAcmClass<'d, usb::Driver<'d, peripherals::USB>>;

async fn run_session(mut cdc_acm: CdcAcm<'_>, name: &str) -> Result<(), usb_driver::EndpointError> {
    let mut first = true;
    let mut buf = [0u8; 64];
    let mut text_opening = String::<64>::new();
    write!(text_opening, "\r\nEcho via {}\r\n", name).unwrap();
    loop {
        cdc_acm.wait_connection().await;
        info!("Connected");
        loop {
            let n = cdc_acm.read_packet(&mut buf).await?;
            if first {
                cdc_acm.write_packet(text_opening.as_bytes()).await?;
                first = false;
            }
            cdc_acm.write_packet(&buf[..n]).await?;
        }
    }
    #[allow(unreachable_code)]
    Ok(())
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

impl embassy_usb::Handler for DeviceHandler {
    /// Called when the USB device has been enabled or disabled.
    fn enabled(&mut self, enabled: bool) {
        info!("embassy_usb::Handler.enabled({})", enabled);
        self.configured.store(false, atomic::Ordering::Relaxed);
    }
    /// Called after a USB reset after the bus reset sequence is complete.
    fn reset(&mut self) {
        info!("embassy_usb::Handler.reset()");
        self.configured.store(false, atomic::Ordering::Relaxed);
    }
    /// Called when the host has set the address of the device to `addr`.
    fn addressed(&mut self, addr: u8) {
        info!("embassy_usb::Handler.addressed(addr: {})", addr);
        self.configured.store(false, atomic::Ordering::Relaxed);
    }
    /// Called when the host has enabled or disabled the configuration of the device.
    fn configured(&mut self, configured: bool) {
        info!("embassy_usb::Handler.configured(configured: {})", configured);
        self.configured.store(configured, atomic::Ordering::Relaxed);
    }
}
