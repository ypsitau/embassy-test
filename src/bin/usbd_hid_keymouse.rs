#![no_std]
#![no_main]

use core::sync::atomic;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_time::Timer;
use embassy_usb as usb;
use usbd_hid::descriptor::SerializedDescriptor;
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
        const VID: u16 = 0xc0de;
        const PID: u16 = 0xcafe;
        const CONFIG_DESCRIPTOR_SIZE: usize = 256;
        const BOS_DESCRIPTOR_SIZE: usize = 256;
        const MSOS_DESCRIPTOR_SIZE: usize = 256;
        const CONTROL_BUF_SIZE: usize = 64;
        let mut config = usb::Config::new(VID, PID);
        config.manufacturer = Some("Embassy");
        config.product = Some("usbd_hid_keymouse");
        config.serial_number = Some("12345678");
        config.max_power = 100;
        config.max_packet_size_0 = CONTROL_BUF_SIZE as u8;
        config.composite_with_iads = false;
        config.device_class = 0;
        config.device_sub_class = 0;
        config.device_protocol = 0;
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
    let (hid_keyboard_reader, mut hid_keyboard_writer) = {
        let state = {
            static STATIC_CELL: StaticCell<usb::class::hid::State> = StaticCell::new();
            STATIC_CELL.init(usb::class::hid::State::new())
        };
        let config = usb::class::hid::Config {
            report_descriptor: usbd_hid::descriptor::KeyboardReport::desc(),
            request_handler: None,
            poll_ms: 60,
            max_packet_size: 64,
            hid_subclass: usb::class::hid::HidSubclass::Boot,
            hid_boot_protocol: usb::class::hid::HidBootProtocol::Keyboard,
        };
        usb::class::hid::HidReaderWriter::<_, 1, 8>::new(&mut usb_builder, state, config).split()
    };
    let (hid_mouse_reader, mut hid_mouse_writer) = {
        let state = {
            static STATIC_CELL: StaticCell<usb::class::hid::State> = StaticCell::new();
            STATIC_CELL.init(usb::class::hid::State::new())
        };
        let config = usb::class::hid::Config {
            report_descriptor: usbd_hid::descriptor::MouseReport::desc(),
            request_handler: None,
            poll_ms: 60,
            max_packet_size: 64,
            hid_subclass: usb::class::hid::HidSubclass::Boot,
            hid_boot_protocol: usb::class::hid::HidBootProtocol::Mouse,
        };
        usb::class::hid::HidReaderWriter::<_, 1, 8>::new(&mut usb_builder, state, config).split()
    };
    let mut usb_device = usb_builder.build();
    let fut_usb = usb_device.run();
    let fut_hid_keyboard_writer = async {
        let mut gpio_button_left = rp::gpio::Input::new(p.PIN_18, rp::gpio::Pull::Up);
        let mut gpio_button_up = rp::gpio::Input::new(p.PIN_19, rp::gpio::Pull::Up);
        let mut gpio_button_down = rp::gpio::Input::new(p.PIN_20, rp::gpio::Pull::Up);
        let mut gpio_button_right = rp::gpio::Input::new(p.PIN_21, rp::gpio::Pull::Up);
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
        let gpio_button_a = rp::gpio::Input::new(p.PIN_16, rp::gpio::Pull::Up);
        let gpio_button_b = rp::gpio::Input::new(p.PIN_17, rp::gpio::Pull::Up);
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
        let mut hid_request_handler = HidRequestHandler::new();
        hid_keyboard_reader.run(false, &mut hid_request_handler).await;
    };
    let fut_hid_mouse_reader = async {
        let mut hid_request_handler = HidRequestHandler::new();
        hid_mouse_reader.run(false, &mut hid_request_handler).await;
    };
    embassy_futures::join::join5(fut_usb, fut_hid_keyboard_writer, fut_hid_mouse_writer, fut_hid_keyboard_reader, fut_hid_mouse_reader).await;
}

//-----------------------------------------------------------------------------
// HidRequestHandler
//-----------------------------------------------------------------------------
struct HidRequestHandler {
    hid_protocol_mode: atomic::AtomicU8,
}

impl HidRequestHandler {
    fn new() -> Self {
        HidRequestHandler {
            hid_protocol_mode: atomic::AtomicU8::new(usb::class::hid::HidProtocolMode::Boot as u8),
        }
    }
}

impl usb::class::hid::RequestHandler for HidRequestHandler {
    // Reads the value of report `id` into `buf` returning the size.
    // Returns `None` if `id` is invalid or no data is available.
    fn get_report(&mut self, id: usb::class::hid::ReportId, _buf: &mut [u8]) -> Option<usize> {
        info!("hid::RequestHander.get_report(id: {:?})", id);
        None
    }
    // Sets the value of report `id` to `data`.
    fn set_report(&mut self, id: usb::class::hid::ReportId, data: &[u8]) -> usb::control::OutResponse {
        info!("hid::RequestHandler.set_report(id: {:?}, data: {:02x}", id, data);
        usb::control::OutResponse::Accepted
    }
    // Gets the current hid protocol.
    // Returns `Report` protocol by default.
    fn get_protocol(&self) -> usb::class::hid::HidProtocolMode {
        let mode = usb::class::hid::HidProtocolMode::from(self.hid_protocol_mode.load(atomic::Ordering::Relaxed));
        info!("hid::RequestHandler.get_protocol() -> {}", mode);
        mode
    }
    // Sets the current hid protocol to `protocol`.
    // Accepts only `Report` protocol by default.
    fn set_protocol(&mut self, protocol: usb::class::hid::HidProtocolMode) -> usb::control::OutResponse {
        info!("hid::RequestHandler.set_protocol(protocol: {})", protocol);
        self.hid_protocol_mode.store(protocol as u8, atomic::Ordering::Relaxed);
        usb::control::OutResponse::Accepted
    }
    // Get the idle rate for `id`.
    // If `id` is `None`, get the idle rate for all reports. Returning `None`
    // will reject the control request. Any duration at or above 1.024 seconds
    // or below 4ms will be returned as an indefinite idle rate.
    fn get_idle_ms(&mut self, id: Option<usb::class::hid::ReportId>) -> Option<u32> {
        info!("hid::RequestHandler.get_idle_ms(id: {:?})", id);
        None
    }
    // Set the idle rate for `id` to `dur`.
    // If `id` is `None`, set the idle rate of all input reports to `dur`. If
    // an indefinite duration is requested, `dur` will be set to `u32::MAX`.
    fn set_idle_ms(&mut self, id: Option<usb::class::hid::ReportId>, dur: u32) {
        info!("hid::RequestHandler.set_idle_ms(id: {:?}, dur: {:?})", id, dur);
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
