//! This example uses the RP Pico W board Wifi chip (cyw43).
//! Connects to specified Wifi network and creates a TCP endpoint on port 1234.

#![no_std]
#![no_main]

use core::str::from_utf8;
use defmt::*;
use embassy_executor::Spawner;
use embassy_net as net;
use embassy_rp as rp;
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

mod private_info;

rp::bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => rp::pio::InterruptHandler<rp::peripherals::PIO0>;
    DMA_IRQ_0 => rp::dma::InterruptHandler<rp::peripherals::DMA_CH0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let (net_driver, mut cyw43_control, cyw43_runner, cyw43_clm) =  {
        let pin_pwr = p.PIN_23;
        let pin_dio = p.PIN_24;
        let pin_cs = p.PIN_25;
        let pin_clk = p.PIN_29;
        let state = {
            static STATIC_CELL: StaticCell<cyw43::State> = StaticCell::new();
            STATIC_CELL.init(cyw43::State::new())
        };
        let pwr = rp::gpio::Output::new(pin_pwr, rp::gpio::Level::Low);
        let spi = {
            let mut pio = rp::pio::Pio::new(p.PIO0, Irqs);
            let sm = pio.sm0;
            let clock_divider = cyw43_pio::DEFAULT_CLOCK_DIVIDER;
            let irq = pio.irq0;
            let cs = rp::gpio::Output::new(pin_cs, rp::gpio::Level::High);
            let dma = rp::dma::Channel::new(p.DMA_CH0, Irqs);
            cyw43_pio::PioSpi::new(&mut pio.common, sm, clock_divider, irq, cs, pin_dio, pin_clk, dma)
        };
        // To make flashing faster for development, you may want to flash the firmwares independently
        // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
        //     probe-rs download 43439A0.bin --binary-format bin --chip RP2040 --base-address 0x10100000
        //     probe-rs download 43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x10140000
        //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
        //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };
        let fw = cyw43::aligned_bytes!("../../cyw43-firmware/43439A0.bin");
        let clm = cyw43::aligned_bytes!("../../cyw43-firmware/43439A0_clm.bin");
        let nvram = cyw43::aligned_bytes!("../../cyw43-firmware/nvram_rp2040.bin");
        let (net_driver, control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;
        (net_driver, control, runner, clm)
    };
    let (net_stack, mut net_runner) = {
        let config = net::Config::dhcpv4(Default::default());
        //let config = {
        //    let address = net::Ipv4Cidr::new(net::Ipv4Address::new(192, 168, 69, 2), 24);
        //    let mut dns_servers = heapless::Vec::<net::Ipv4Address, 3>::new();
        //    dns_servers.push(net::Ipv4Address::new(8, 8, 8, 8)).unwrap();
        //    let gateway = Some(net::Ipv4Address::new(192, 168, 69, 1));
        //    net::Config::ipv4_static(net::StaticConfigV4 {address, dns_servers, gateway})
        //};
        let resources = {
            static STATIC_CELL: StaticCell<net::StackResources<3>> = StaticCell::new();
            STATIC_CELL.init(net::StackResources::new())
        };
        let mut rng = rp::clocks::RoscRng;
        let seed = rng.next_u64();
        net::new(net_driver, config, resources, seed)
    };
    let fut_cys43_runner = cyw43_runner.run();
    let fut_net_runner = net_runner.run();
    let fut_main = async {
        cyw43_control.init(cyw43_clm).await;
        cyw43_control.set_power_management(cyw43::PowerManagementMode::PowerSave).await;
        while let Err(err) = cyw43_control
            .join(private_info::WIFI_NETWORK, cyw43::JoinOptions::new(private_info::WIFI_PASSWORD.as_bytes()))
            .await
        {
            info!("join failed: {:?}", err);
        }
        info!("waiting for link...");
        net_stack.wait_link_up().await;
        info!("waiting for DHCP...");
        net_stack.wait_config_up().await;
        info!("Stack is up!");
        run_http_client(net_stack).await;
    };
    embassy_futures::join::join3(fut_main, fut_cys43_runner, fut_net_runner).await;
}

async fn run_http_client(net_stack: net::Stack<'_>) -> ! {
    let client_state = {
        static STATIC_CELL: StaticCell<net::tcp::client::TcpClientState<1, 4096, 4096>> = StaticCell::new();
        STATIC_CELL.init(net::tcp::client::TcpClientState::<1, 4096, 4096>::new())
    };
    let tls_config = {
        let mut rng = rp::clocks::RoscRng;
        let seed = rng.next_u64();
        let tls_read_buffer = {
            static STATIC_CELL: StaticCell<[u8; 16640]> = StaticCell::new();
            STATIC_CELL.init([0; 16640])
        };
        let tls_write_buffer = {
            static STATIC_CELL: StaticCell<[u8; 16640]> = StaticCell::new();
            STATIC_CELL.init([0; 16640])
        };
        reqwless::client::TlsConfig::new(seed, tls_read_buffer, tls_write_buffer, reqwless::client::TlsVerify::None)
    };
    let rx_buffer = {
        static STATIC_CELL: StaticCell<[u8; 4096]> = StaticCell::new();
        STATIC_CELL.init([0; 4096])
    };
    loop {
        let tcp_client = net::tcp::client::TcpClient::new(net_stack, client_state);
        let dns_client = net::dns::DnsSocket::new(net_stack);
        let mut http_client = reqwless::client::HttpClient::new(&tcp_client, &dns_client);
        let url = "http://httpbin.org/json";
        //let mut http_client = reqwless::client::HttpClient::new_with_tls(&tcp_client, &dns_client, tls_config);
        //let url = "https://httpbin.org/json";
        info!("connecting to {}", &url);
        let mut request = match http_client.request(reqwless::request::Method::GET, url).await {
            Ok(request) => request,
            Err(e) => {
                error!("Failed to make HTTP request: {:?}", e);
                Timer::after(Duration::from_secs(5)).await;
                continue;
            }
        };
        let response = match request.send(rx_buffer).await {
            Ok(response) => response,
            Err(e) => {
                error!("Failed to send HTTP request: {:?}", e);
                Timer::after(Duration::from_secs(5)).await;
                continue;
            }
        };
        info!("Response status: {}", response.status.0);
        let body_bytes = match response.body().read_to_end().await {
            Ok(body_bytes) => body_bytes,
            Err(_e) => {
                error!("Failed to read response body");
                Timer::after(Duration::from_secs(5)).await;
                continue;
            }
        };
        let body_utf8 = match from_utf8(body_bytes) {
            Ok(body_utf8) => body_utf8,
            Err(_e) => {
                error!("Failed to parse response body as UTF-8");
                Timer::after(Duration::from_secs(5)).await;
                continue;
            }
        };
        info!("Response body length: {} bytes", body_utf8.len());
        // Parse the JSON response from httpbin.org/json
        #[derive(serde::Deserialize)]
        struct SlideShow<'a> {
            author: &'a str,
            title: &'a str,
        }
        #[derive(serde::Deserialize)]
        struct HttpBinResponse<'a> {
            #[serde(borrow)]
            slideshow: SlideShow<'a>,
        }
        match serde_json_core::from_slice::<HttpBinResponse>(body_utf8.as_bytes()) {
            Ok((output, _used)) => {
                info!("Successfully parsed JSON response!");
                info!("Slideshow title: {:?}", output.slideshow.title);
                info!("Slideshow author: {:?}", output.slideshow.author);
            }
            Err(e) => {
                error!("Failed to parse JSON response: {}", Debug2Format(&e));
                // Log preview of response for debugging
                let preview = if body_utf8.len() > 200 { &body_utf8[..200] } else { body_utf8 };
                info!("Response preview: {:?}", preview);
            }
        }
        Timer::after(Duration::from_secs(5)).await;
    }
}
