#![no_std]
#![no_main]

use defmt::{info, panic};
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals;
use embassy_rp::usb;
use embassy_usb::UsbDevice;
use embassy_usb::class as usb_class;
use embassy_usb::driver as usb_driver;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => usb::InterruptHandler<peripherals::USB>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello there!");
    let p = embassy_rp::init(Default::default());
    let driver = usb::Driver::new(p.USB, Irqs);
    let mut builder = {
        let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
        config.manufacturer = Some("Embassy");
        config.product = Some("USB-serial example");
        config.serial_number = Some("12345678");
        config.max_power = 100;
        config.max_packet_size_0 = 64;
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
        embassy_usb::Builder::new(
            driver,
            config,
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            &mut [], // no msos descriptors
            CONTROL_BUF.init([0; 64]),
        )
    };
    let mut class = {
        static STATE: StaticCell<usb_class::cdc_acm::State> = StaticCell::new();
        let state = STATE.init(usb_class::cdc_acm::State::new());
        let max_packet_size = 64;
        usb_class::cdc_acm::CdcAcmClass::new(&mut builder, state, max_packet_size)
    };
    let usb_device = builder.build();
    spawner.spawn(usb_task(usb_device).unwrap());
    loop {
        class.wait_connection().await;
        info!("Connected");
        let mut buf = [0; 64];
        let err = loop {
            match class.read_packet(&mut buf).await {
                Ok(n) => {
                    let data = &buf[..n];
                    info!("data: {:x}", data);
                    if let Err(err) = class.write_packet(data).await { break err; };
                },
                Err(err) => break err,
            }
        };
        match err {
            usb_driver::EndpointError::BufferOverflow => panic!("Buffer overflow"),
            usb_driver::EndpointError::Disabled => info!("Disconnected"),
        }
    }
}

#[embassy_executor::task]
async fn usb_task(mut usb_device: UsbDevice<'static, usb::Driver<'static, peripherals::USB>>) -> ! {
    usb_device.run().await
}
