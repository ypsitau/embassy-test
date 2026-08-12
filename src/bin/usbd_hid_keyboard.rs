#![no_std]
#![no_main]

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::join::join;
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

static HID_PROTOCOL_MODE: AtomicU8 = AtomicU8::new(usb_class::hid::HidProtocolMode::Boot as u8);

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let driver = usb::Driver::new(p.USB, Irqs);
    let mut request_handler = MyRequestHandler {};
    let mut builder = {
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
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static MSOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
        static DEVICE_HANDLER: StaticCell<MyDeviceHandler> = StaticCell::new();
        let mut builder = embassy_usb::Builder::new(
            driver,
            config,
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            MSOS_DESCRIPTOR.init([0; 256]),
            CONTROL_BUF.init([0; 64]),
        );
        builder.handler(DEVICE_HANDLER.init(MyDeviceHandler::new()));
        builder
    };
    let hid = {
        static STATE: StaticCell<usb_class::hid::State> = StaticCell::new();
        let config = embassy_usb::class::hid::Config {
            report_descriptor: usbd_hid::descriptor::KeyboardReport::desc(),
            request_handler: None,
            poll_ms: 60,
            max_packet_size: 64,
            hid_subclass: usb_class::hid::HidSubclass::Boot,
            hid_boot_protocol: usb_class::hid::HidBootProtocol::Keyboard,
        };
        usb_class::hid::HidReaderWriter::<_, 1, 8>::new(&mut builder,
                STATE.init(usb_class::hid::State::new()), config)
    };
    let mut usb = builder.build();
    let usb_fut = usb.run();
    let mut gpio_signal = gpio::Input::new(p.PIN_16, gpio::Pull::Up);
    let (reader, mut writer) = hid.split();
    let in_fut = async {
        loop {
            gpio_signal.wait_for_any_edge().await;
            Timer::after_millis(100).await; // skip the bounding period
            let key_code = match gpio_signal.get_level() {
                gpio::Level::High => { info!("HIGH DETECTED"); 0 },
                gpio::Level::Low => { info!("LOW DETECTED"); 4 },
            };
            if HID_PROTOCOL_MODE.load(Ordering::Relaxed) == usb_class::hid::HidProtocolMode::Boot as u8 {
                if let Err(e) = writer.write(&[0, 0, key_code, 0, 0, 0, 0, 0]).await {
                    warn!("Failed to send boot report: {:?}", e);
                }
            } else {
                let report = usbd_hid::descriptor::KeyboardReport {
                    keycodes: [key_code, 0, 0, 0, 0, 0],
                    leds: 0,
                    modifier: 0,
                    reserved: 0,
                };
                if let Err(e) = writer.write_serialize(&report).await {
                    warn!("Failed to send report: {:?}", e);
                }
            }
        }
    };

    let out_fut = async {
        reader.run(false, &mut request_handler).await;
    };

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join(usb_fut, join(in_fut, out_fut)).await;
}

struct MyRequestHandler {}

impl usb_class::hid::RequestHandler for MyRequestHandler {
    fn get_report(&mut self, id: usb_class::hid::ReportId, _buf: &mut [u8]) -> Option<usize> {
        info!("Get report for {:?}", id);
        None
    }

    fn set_report(&mut self, id: usb_class::hid::ReportId, data: &[u8]) -> embassy_usb::control::OutResponse {
        info!("Set report for {:?}: {=[u8]}", id, data);
        embassy_usb::control::OutResponse::Accepted
    }

    fn get_protocol(&self) -> usb_class::hid::HidProtocolMode {
        let protocol = usb_class::hid::HidProtocolMode::from(HID_PROTOCOL_MODE.load(Ordering::Relaxed));
        info!("The current HID protocol mode is: {}", protocol);
        protocol
    }

    fn set_protocol(&mut self, protocol: usb_class::hid::HidProtocolMode) -> embassy_usb::control::OutResponse {
        info!("Switching to HID protocol mode: {}", protocol);
        HID_PROTOCOL_MODE.store(protocol as u8, Ordering::Relaxed);
        embassy_usb::control::OutResponse::Accepted
    }

    fn set_idle_ms(&mut self, id: Option<usb_class::hid::ReportId>, dur: u32) {
        info!("Set idle rate for {:?} to {:?}", id, dur);
    }

    fn get_idle_ms(&mut self, id: Option<usb_class::hid::ReportId>) -> Option<u32> {
        info!("Get idle rate for {:?}", id);
        None
    }
}

struct MyDeviceHandler {
    configured: AtomicBool,
}

impl MyDeviceHandler {
    fn new() -> Self {
        MyDeviceHandler {
            configured: AtomicBool::new(false),
        }
    }
}

impl embassy_usb::Handler for MyDeviceHandler {
    fn enabled(&mut self, enabled: bool) {
        self.configured.store(false, Ordering::Relaxed);
        if enabled {
            info!("Device enabled");
        } else {
            info!("Device disabled");
        }
    }

    fn reset(&mut self) {
        self.configured.store(false, Ordering::Relaxed);
        info!("Bus reset, the Vbus current limit is 100mA");
    }

    fn addressed(&mut self, addr: u8) {
        self.configured.store(false, Ordering::Relaxed);
        info!("USB address set to: {}", addr);
    }

    fn configured(&mut self, configured: bool) {
        self.configured.store(configured, Ordering::Relaxed);
        if configured {
            info!("Device configured, it may now draw up to the configured current limit from Vbus.")
        } else {
            info!("Device is no longer configured, the Vbus current limit is 100mA.");
        }
    }
}
