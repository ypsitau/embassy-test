//! This example uses the RP Pico W board Wifi chip (cyw43).
//! Scans Wifi for ssid names.

#![no_std]
#![no_main]

use core::str;
use defmt::*;
use embassy_rp as rp;
use embassy_executor::Spawner;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

rp::bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => rp::pio::InterruptHandler<rp::peripherals::PIO0>;
    DMA_IRQ_0 => rp::dma::InterruptHandler<rp::peripherals::DMA_CH0>, rp::dma::InterruptHandler<rp::peripherals::DMA_CH1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Hello World!");

    let p = embassy_rp::init(Default::default());
    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --binary-format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download 43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let (_net_device, mut control, runner, clm) =  {
        let state = {
            static STATIC_CELL: StaticCell<cyw43::State> = StaticCell::new();
            STATIC_CELL.init(cyw43::State::new())
        };
        let pwr = rp::gpio::Output::new(p.PIN_23, rp::gpio::Level::Low);
        let spi = {
            let mut pio = rp::pio::Pio::new(p.PIO0, Irqs);
            let sm = pio.sm0;
            let clock_divider = cyw43_pio::DEFAULT_CLOCK_DIVIDER;
            let irq = pio.irq0;
            let cs = rp::gpio::Output::new(p.PIN_25, rp::gpio::Level::High);
            let pin_dio = p.PIN_24;
            let pin_clk = p.PIN_29;
            let dma = rp::dma::Channel::new(p.DMA_CH0, Irqs);
            cyw43_pio::PioSpi::new(&mut pio.common, sm, clock_divider, irq, cs, pin_dio, pin_clk, dma)
        };
        let fw = cyw43::aligned_bytes!("../../cyw43-firmware/43439A0.bin");
        let clm = cyw43::aligned_bytes!("../../cyw43-firmware/43439A0_clm.bin");
        let nvram = cyw43::aligned_bytes!("../../cyw43-firmware/nvram_rp2040.bin");
        let (net_device, control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;
        (net_device, control, runner, clm)
    };
    let fut_runner = runner.run();
    let fut_scan = async {
        control.init(clm).await;
        control.set_power_management(cyw43::PowerManagementMode::PowerSave).await;
        let mut scanner = control.scan(Default::default()).await;
        while let Some(bss) = scanner.next().await {
            if let Ok(ssid_str) = str::from_utf8(&bss.ssid) {
                info!("scanned {} == {:x}", ssid_str, bss.bssid);
            }
        }
    };
    embassy_futures::join::join(fut_scan, fut_runner).await;
}
