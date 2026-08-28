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

const FLASH_OFFSET_TOP: u32 = 0x100000;
const FLASH_SIZE: usize = 2 * 1024 * 1024;
type Flash = rp::flash::Flash<'static, rp::peripherals::FLASH, rp::flash::Async, FLASH_SIZE>;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    info!("Hello World!");
    let mut flash = Flash::new(p.FLASH, p.DMA_CH0, Irqs);
    // Get JEDEC id
    let jedec = flash.blocking_jedec_id().unwrap();
    info!("jedec id: 0x{:x}", jedec);
    // Get unique id
    let mut uid = [0u8; 8];
    flash.blocking_unique_id(&mut uid).unwrap();
    info!("unique id: {=[u8]:02x}", uid);
    run_erase_write_sector(&mut flash, (rp::flash::ERASE_SIZE * 0) as u32);
    run_multiwrite_bytes(&mut flash, (rp::flash::ERASE_SIZE * 1) as u32);
    run_background_read(&mut flash, (rp::flash::ERASE_SIZE * 2) as u32).await;
    loop {}
}

fn run_erase_write_sector(flash: &mut Flash, offset: u32) {
    let flash_offset = FLASH_OFFSET_TOP + offset;
    info!(">>>> [erase_write_sector]");
    let mut buf = [0u8; rp::flash::ERASE_SIZE];
    flash.blocking_read(flash_offset, &mut buf).unwrap();
    info!("Addr of flash block is {:08x}", flash_offset + rp::flash::FLASH_BASE as u32);
    info!("The first 4 bytes: {=[u8]:02x}", buf[0..4]);
    flash.blocking_erase(flash_offset, flash_offset + rp::flash::ERASE_SIZE as u32).unwrap();
    flash.blocking_read(flash_offset, &mut buf).unwrap();
    info!("The first 4 bytes after erase: {=[u8]:02x}", buf[0..4]);
    if buf.iter().any(|pbuf| *pbuf != 0xff) {
        defmt::panic!("unexpected");
    }
    for pbuf in buf.iter_mut() {
        *pbuf = 0xda;
    }
    flash.blocking_write(flash_offset, &buf).unwrap();
    flash.blocking_read(flash_offset, &mut buf).unwrap();
    info!("The first 4 bytes after write: {=[u8]:02x}", buf[0..4]);
    if buf.iter().any(|pbuf| *pbuf != 0xda) {
        defmt::panic!("unexpected");
    }
}

fn run_multiwrite_bytes(flash: &mut Flash, offset: u32) {
    let flash_offset = FLASH_OFFSET_TOP + offset;
    info!(">>>> [multiwrite_bytes]");
    let mut buf = [0u8; rp::flash::ERASE_SIZE];
    flash.blocking_read(flash_offset, &mut buf).unwrap();
    info!("Addr of flash block is {:08x}", flash_offset + rp::flash::FLASH_BASE as u32);
    info!("The first 4 bytes: {=[u8]:02x}", buf[0..4]);
    flash.blocking_erase(flash_offset, flash_offset + rp::flash::ERASE_SIZE as u32).unwrap();
    flash.blocking_read(flash_offset, &mut buf).unwrap();
    info!("The first 4 bytes after erase: {=[u8]:02x}", buf[0..4]);
    if buf.iter().any(|pbuf| *pbuf != 0xff) {
        defmt::panic!("unexpected");
    }
    flash.blocking_write(flash_offset + 0, &[0x01]).unwrap();
    flash.blocking_write(flash_offset + 1, &[0x02]).unwrap();
    flash.blocking_write(flash_offset + 2, &[0x03]).unwrap();
    flash.blocking_write(flash_offset + 3, &[0x04]).unwrap();
    flash.blocking_read(flash_offset, &mut buf).unwrap();
    info!("The first 4 bytes after write: {=[u8]:02x}", buf[0..4]);
    if &buf[0..4] != &[0x01, 0x02, 0x03, 0x04] {
        defmt::panic!("unexpected");
    }
}

async fn run_background_read(flash: &mut Flash, offset: u32) {
    let flash_offset = FLASH_OFFSET_TOP + offset;
    info!(">>>> [background_read]");
    let mut buf = [0u32; 8];
    flash.background_read(flash_offset, &mut buf).unwrap().await;
    info!("Addr of flash block is {:08x}", flash_offset + rp::flash::FLASH_BASE as u32);
    info!("The first 4 bytes: {=u32:08x}", buf[0]);
    flash.blocking_erase(flash_offset, flash_offset + rp::flash::ERASE_SIZE as u32).unwrap();
    flash.background_read(flash_offset, &mut buf).unwrap().await;
    info!("The first 4 bytes after erase: {=u32:08x}", buf[0]);
    if buf.iter().any(|pbuf| *pbuf != 0xffffffff) {
        defmt::panic!("unexpected");
    }
    for pbuf in buf.iter_mut() {
        *pbuf = 0xdaba1234;
    }
    flash.blocking_write(flash_offset, unsafe {
        core::slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len() * 4)
    }).unwrap();
    flash.background_read(flash_offset, &mut buf).unwrap().await;
    info!("The first 4 bytes after write: {=u32:08x}", buf[0]);
    if buf.iter().any(|pbuf| *pbuf != 0xdaba1234) {
        defmt::panic!("unexpected");
    }
}
