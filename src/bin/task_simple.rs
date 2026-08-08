#![no_std]
#![no_main]

use defmt::{info, Format};
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::Channel};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[derive(Debug, Format)]
struct Packet(u8, u8);

type PacketChannel = Channel<ThreadModeRawMutex, Packet, 4>;

#[embassy_executor::task]
async fn sender(packet_channel: &'static PacketChannel) {
    for value in 0..10 {
        let packet = Packet(value, value);
        info!("sent: {:?}", packet);
        packet_channel.send(packet).await;
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn receiver(packet_channel: &'static PacketChannel) {
    loop {
        let packet = packet_channel.receive().await;
        info!("received: {:?}", packet);
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let _ = embassy_rp::init(Default::default());

    static CHANNEL: PacketChannel = Channel::new();

    spawner.spawn(sender(&CHANNEL).unwrap());
    spawner.spawn(receiver(&CHANNEL).unwrap());

    loop {
        Timer::after_secs(5).await;
    }
}
