#![no_std]
#![no_main]

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio;
use embassy_rp::peripherals;
use embassy_rp::usb;
use embassy_time::Timer;
use embassy_usb::class as usb_class;
use usbd_hid::descriptor::SerializedDescriptor;
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
        config.product = Some("HID keyboard example");
        config.serial_number = Some("12345678");
        config.max_power = 100;
        config.max_packet_size_0 = 64;
        config.composite_with_iads = false;
        config.device_class = 0;
        config.device_sub_class = 0;
        config.device_protocol = 0;
        static CONFIG_DESCRIPTOR_BUF: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR_BUF: StaticCell<[u8; 256]> = StaticCell::new();
        static MSOS_DESCRIPTOR_BUF: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
        let mut usb_builder = embassy_usb::Builder::new(
            usb_driver,
            config,
            CONFIG_DESCRIPTOR_BUF.init([0; 256]),
            BOS_DESCRIPTOR_BUF.init([0; 256]),
            MSOS_DESCRIPTOR_BUF.init([0; 256]),
            CONTROL_BUF.init([0; 64]),
        );
        static DEVICE_HANDLER: StaticCell<DeviceHandler> = StaticCell::new();
        usb_builder.handler(DEVICE_HANDLER.init(DeviceHandler::new()));
        usb_builder
    };
    let (hid_keyboard_reader, mut hid_keyboard_writer) = {
        static STATE: StaticCell<usb_class::hid::State> = StaticCell::new();
        let config = embassy_usb::class::hid::Config {
            report_descriptor: usbd_hid::descriptor::KeyboardReport::desc(),
            request_handler: None,
            poll_ms: 60,
            max_packet_size: 64,
            hid_subclass: usb_class::hid::HidSubclass::Boot,
            hid_boot_protocol: usb_class::hid::HidBootProtocol::Keyboard,
        };
        usb_class::hid::HidReaderWriter::<_, 1, 8>::new(
            &mut usb_builder,
            STATE.init(usb_class::hid::State::new()),
            config,
        ).split()
    };
    let (hid_mouse_reader, mut hid_mouse_writer) = {
        static STATE: StaticCell<usb_class::hid::State> = StaticCell::new();
        let config = embassy_usb::class::hid::Config {
            report_descriptor: usbd_hid::descriptor::MouseReport::desc(),
            request_handler: None,
            poll_ms: 60,
            max_packet_size: 64,
            hid_subclass: usb_class::hid::HidSubclass::Boot,
            hid_boot_protocol: usb_class::hid::HidBootProtocol::Mouse,
        };
        usb_class::hid::HidReaderWriter::<_, 1, 8>::new(
            &mut usb_builder,
            STATE.init(usb_class::hid::State::new()),
            config,
        ).split()
    };
    let mut usb_device = usb_builder.build();
    let fut_usb = usb_device.run();
    let fut_hid_keyboard_writer = async {
        let mut gpio_button_left = gpio::Input::new(p.PIN_18, gpio::Pull::Up);
        let mut gpio_button_up = gpio::Input::new(p.PIN_19, gpio::Pull::Up);
        let mut gpio_button_down = gpio::Input::new(p.PIN_20, gpio::Pull::Up);
        let mut gpio_button_right = gpio::Input::new(p.PIN_21, gpio::Pull::Up);
        loop {
            embassy_futures::select::select_array([
                gpio_button_left.wait_for_any_edge(),
                gpio_button_up.wait_for_any_edge(),
                gpio_button_down.wait_for_any_edge(),
                gpio_button_right.wait_for_any_edge(),
            ]).await;
            Timer::after_millis(30).await; // skip the bounding period
            let mut n_keycodes = 0;
            let mut keycodes: [u8; 6] = [0; 6];
            if gpio_button_left.is_low() { keycodes[n_keycodes] = 0x50; n_keycodes += 1; }
            if gpio_button_up.is_low() { keycodes[n_keycodes] = 0x52; n_keycodes += 1; }
            if gpio_button_down.is_low() { keycodes[n_keycodes] = 0x51; n_keycodes += 1; }
            if gpio_button_right.is_low() { keycodes[n_keycodes] = 0x4f; /* n_keycodes += 1; */ }
            let keyboard_report = usbd_hid::descriptor::KeyboardReport {
                modifier: 0,
                reserved: 0,
                leds: 0,
                keycodes,
            };
            //let mut buf: [u8; 8] = [0; 8];
            //if let Ok(len) = keyboard_report.serialize(&mut buf) {
            //    info!("Serialized report: {:?}", &buf[..len]);
            //}
            if let Err(e) = hid_keyboard_writer.write_serialize(&keyboard_report).await {
                warn!("Failed to send report: {:?}", e);
            }
        }
    };
    let fut_hid_mouse_writer = async {
        let gpio_button_a = gpio::Input::new(p.PIN_16, gpio::Pull::Up);
        let gpio_button_b = gpio::Input::new(p.PIN_17, gpio::Pull::Up);
        loop {
            let mouse_report = usbd_hid::descriptor::MouseReport {
                buttons: 0,
                x: 0,
                y: if gpio_button_a.is_low() { -20 } else if gpio_button_b.is_low() { 20 } else { 0 },
                wheel: 0,
                pan: 0,
            };
            if let Err(e) = hid_mouse_writer.write_serialize(&mouse_report).await {
                warn!("Failed to send report: {:?}", e);
            }
            Timer::after_millis(100).await;
        }
    };
    let fut_hid_keyboard_reader = async {
        let mut request_handler = RequestHandler::new();
        hid_keyboard_reader.run(false, &mut request_handler).await;
    };
    let fut_hid_mouse_reader = async {
        let mut request_handler = RequestHandler::new();
        hid_mouse_reader.run(false, &mut request_handler).await;
    };
    embassy_futures::join::join5(fut_usb, fut_hid_keyboard_writer, fut_hid_mouse_writer, fut_hid_keyboard_reader, fut_hid_mouse_reader).await;
}

//-----------------------------------------------------------------------------
// RequestHandler
//-----------------------------------------------------------------------------
struct RequestHandler {
    hid_protocol_mode: AtomicU8,
}

impl RequestHandler {
    fn new() -> Self {
        RequestHandler {
            hid_protocol_mode: AtomicU8::new(usb_class::hid::HidProtocolMode::Boot as u8),
        }
    }
}

impl usb_class::hid::RequestHandler for RequestHandler {
    // Reads the value of report `id` into `buf` returning the size.
    // Returns `None` if `id` is invalid or no data is available.
    fn get_report(&mut self, id: usb_class::hid::ReportId, _buf: &mut [u8]) -> Option<usize> {
        info!("hid::RequestHander.get_report(id: {:?})", id);
        None
    }
    // Sets the value of report `id` to `data`.
    fn set_report(&mut self, id: usb_class::hid::ReportId, data: &[u8]) -> embassy_usb::control::OutResponse {
        info!("hid::RequestHandler.set_report(id: {:?}, data: {=[u8]})", id, data);
        embassy_usb::control::OutResponse::Accepted
    }
    // Gets the current hid protocol.
    // Returns `Report` protocol by default.
    fn get_protocol(&self) -> usb_class::hid::HidProtocolMode {
        let mode = usb_class::hid::HidProtocolMode::from(self.hid_protocol_mode.load(Ordering::Relaxed));
        info!("hid::RequestHandler.get_protocol() -> {}", mode);
        mode
    }
    // Sets the current hid protocol to `protocol`.
    // Accepts only `Report` protocol by default.
    fn set_protocol(&mut self, protocol: usb_class::hid::HidProtocolMode) -> embassy_usb::control::OutResponse {
        info!("hid::RequestHandler.set_protocol(protocol: {})", protocol);
        self.hid_protocol_mode.store(protocol as u8, Ordering::Relaxed);
        embassy_usb::control::OutResponse::Accepted
    }
    // Get the idle rate for `id`.
    // If `id` is `None`, get the idle rate for all reports. Returning `None`
    // will reject the control request. Any duration at or above 1.024 seconds
    // or below 4ms will be returned as an indefinite idle rate.
    fn get_idle_ms(&mut self, id: Option<usb_class::hid::ReportId>) -> Option<u32> {
        info!("hid::RequestHandler.get_idle_ms(id: {:?})", id);
        None
    }
    // Set the idle rate for `id` to `dur`.
    // If `id` is `None`, set the idle rate of all input reports to `dur`. If
    // an indefinite duration is requested, `dur` will be set to `u32::MAX`.
    fn set_idle_ms(&mut self, id: Option<usb_class::hid::ReportId>, dur: u32) {
        info!("hid::RequestHandler.set_idle_ms(id: {:?}, dur: {:?})", id, dur);
    }
}

//-----------------------------------------------------------------------------
// DeviceHandler
//-----------------------------------------------------------------------------
struct DeviceHandler {
    configured: AtomicBool,
}

impl DeviceHandler {
    fn new() -> Self {
        DeviceHandler {
            configured: AtomicBool::new(false),
        }
    }
}

impl embassy_usb::Handler for DeviceHandler {
    /// Called when the USB device has been enabled or disabled.
    fn enabled(&mut self, enabled: bool) {
        info!("embassy_usb::Handler.enabled({})", enabled);
        self.configured.store(false, Ordering::Relaxed);
    }
    /// Called after a USB reset after the bus reset sequence is complete.
    fn reset(&mut self) {
        info!("embassy_usb::Handler.reset()");
        self.configured.store(false, Ordering::Relaxed);
    }
    /// Called when the host has set the address of the device to `addr`.
    fn addressed(&mut self, addr: u8) {
        info!("embassy_usb::Handler.addressed(addr: {})", addr);
        self.configured.store(false, Ordering::Relaxed);
    }
    /// Called when the host has enabled or disabled the configuration of the device.
    fn configured(&mut self, configured: bool) {
        info!("embassy_usb::Handler.configured(configured: {})", configured);
        self.configured.store(configured, Ordering::Relaxed);
    }
}
