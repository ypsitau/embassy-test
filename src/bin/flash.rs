//! This example test the flash connected to the RP2040 chip.

#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp as rp;
use {defmt_rtt as _, panic_probe as _};

rp::bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => rp::dma::InterruptHandler<rp::peripherals::DMA_CH0>;
});

const ADDR_OFFSET: u32 = 0x100000;
const FLASH_SIZE: usize = 2 * 1024 * 1024;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    info!("Hello World!");
    let mut flash = rp::flash::Flash::<_, rp::flash::Async, FLASH_SIZE>::new(p.FLASH, p.DMA_CH0, Irqs);
    // Get JEDEC id
    let jedec = flash.blocking_jedec_id().unwrap();
    info!("jedec id: 0x{:x}", jedec);
    // Get unique id
    let mut uid = [0; 8];
    flash.blocking_unique_id(&mut uid).unwrap();
    info!("unique id: {:?}", uid);
    run_erase_write_sector(&mut flash, 0x00);
    run_multiwrite_bytes(&mut flash, rp::flash::ERASE_SIZE as u32);
    run_background_read(&mut flash, (rp::flash::ERASE_SIZE * 2) as u32).await;
    loop {}
}

fn run_erase_write_sector(flash: &mut rp::flash::Flash<'_, rp::peripherals::FLASH, rp::flash::Async, FLASH_SIZE>, offset: u32) {
    info!(">>>> [erase_write_sector]");
    let mut buf = [0u8; rp::flash::ERASE_SIZE];
    flash.blocking_read(ADDR_OFFSET + offset, &mut buf).unwrap();

    info!("Addr of flash block is {:x}", ADDR_OFFSET + offset + rp::flash::FLASH_BASE as u32);
    info!("Contents start with {=[u8]}", buf[0..4]);

    flash.blocking_erase(ADDR_OFFSET + offset, ADDR_OFFSET + offset + rp::flash::ERASE_SIZE as u32).unwrap();

    flash.blocking_read(ADDR_OFFSET + offset, &mut buf).unwrap();
    info!("Contents after erase starts with {=[u8]}", buf[0..4]);
    if buf.iter().any(|x| *x != 0xFF) {
        defmt::panic!("unexpected");
    }

    for b in buf.iter_mut() {
        *b = 0xDA;
    }

    flash.blocking_write(ADDR_OFFSET + offset, &buf).unwrap();

    flash.blocking_read(ADDR_OFFSET + offset, &mut buf).unwrap();
    info!("Contents after write starts with {=[u8]}", buf[0..4]);
    if buf.iter().any(|x| *x != 0xDA) {
        defmt::panic!("unexpected");
    }
}

fn run_multiwrite_bytes(flash: &mut rp::flash::Flash<'_, rp::peripherals::FLASH, rp::flash::Async, FLASH_SIZE>, offset: u32) {
    info!(">>>> [multiwrite_bytes]");
    let mut read_buf = [0u8; rp::flash::ERASE_SIZE];
    flash.blocking_read(ADDR_OFFSET + offset, &mut read_buf).unwrap();

    info!("Addr of flash block is {:x}", ADDR_OFFSET + offset + rp::flash::FLASH_BASE as u32);
    info!("Contents start with {=[u8]}", read_buf[0..4]);

    flash.blocking_erase(ADDR_OFFSET + offset, ADDR_OFFSET + offset + rp::flash::ERASE_SIZE as u32).unwrap();

    flash.blocking_read(ADDR_OFFSET + offset, &mut read_buf).unwrap();
    info!("Contents after erase starts with {=[u8]}", read_buf[0..4]);
    if read_buf.iter().any(|x| *x != 0xFF) {
        defmt::panic!("unexpected");
    }

    flash.blocking_write(ADDR_OFFSET + offset, &[0x01]).unwrap();
    flash.blocking_write(ADDR_OFFSET + offset + 1, &[0x02]).unwrap();
    flash.blocking_write(ADDR_OFFSET + offset + 2, &[0x03]).unwrap();
    flash.blocking_write(ADDR_OFFSET + offset + 3, &[0x04]).unwrap();

    flash.blocking_read(ADDR_OFFSET + offset, &mut read_buf).unwrap();
    info!("Contents after write starts with {=[u8]}", read_buf[0..4]);
    if &read_buf[0..4] != &[0x01, 0x02, 0x03, 0x04] {
        defmt::panic!("unexpected");
    }
}

async fn run_background_read(flash: &mut rp::flash::Flash<'_, rp::peripherals::FLASH, rp::flash::Async, FLASH_SIZE>, offset: u32) {
    info!(">>>> [background_read]");

    let mut buf = [0u32; 8];
    flash.background_read(ADDR_OFFSET + offset, &mut buf).unwrap().await;

    info!("Addr of flash block is {:x}", ADDR_OFFSET + offset + rp::flash::FLASH_BASE as u32);
    info!("Contents start with {=u32:x}", buf[0]);

    flash.blocking_erase(ADDR_OFFSET + offset, ADDR_OFFSET + offset + rp::flash::ERASE_SIZE as u32).unwrap();

    flash.background_read(ADDR_OFFSET + offset, &mut buf).unwrap().await;
    info!("Contents after erase starts with {=u32:x}", buf[0]);
    if buf.iter().any(|x| *x != 0xFFFFFFFF) {
        defmt::panic!("unexpected");
    }

    for b in buf.iter_mut() {
        *b = 0xDABA1234;
    }

    flash.blocking_write(ADDR_OFFSET + offset, unsafe {
        core::slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len() * 4)
    }).unwrap();

    flash.background_read(ADDR_OFFSET + offset, &mut buf).unwrap().await;
    info!("Contents after write starts with {=u32:x}", buf[0]);
    if buf.iter().any(|x| *x != 0xDABA1234) {
        defmt::panic!("unexpected");
    }
}
