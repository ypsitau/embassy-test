//! This example uses the RP Pico W board Wifi chip (cyw43).
//! Connects to specified Wifi network and creates a TCP endpoint on port 1234.

#![no_std]
#![no_main]

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
        const PRE_DOWNLOAD_FIRMWARE: bool = true;
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
        let (fw, cyw43_clm) = if PRE_DOWNLOAD_FIRMWARE {
            // Use pre-downloaded firmware
            // $ probe-rs download cyw43-firmware/43439A0.bin --binary-format bin --chip RP2040 --base-address 0x10180000
            // $ probe-rs download cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x101c0000
            let fw = unsafe {
                &*(core::ptr::slice_from_raw_parts(0x10180000 as *const u8, 231077)
                    as *const cyw43::Aligned<cyw43::A4, [u8]>)
            };
            let cyw43_clm = unsafe {
                &*(core::ptr::slice_from_raw_parts(0x101c0000 as *const u8, 984)
                    as *const cyw43::Aligned<cyw43::A4, [u8]>)
            };
            (fw, cyw43_clm)
        } else {
            let fw = cyw43::aligned_bytes!("../../cyw43-firmware/43439A0.bin");
            let cyw43_clm = cyw43::aligned_bytes!("../../cyw43-firmware/43439A0_clm.bin");
            (fw, cyw43_clm)
        };
        let nvram = cyw43::aligned_bytes!("../../cyw43-firmware/nvram_rp2040.bin");
        let (net_driver, cyw43_control, cyw43_runner) = cyw43::new(state, pwr, spi, fw, nvram).await;
        (net_driver, cyw43_control, cyw43_runner, cyw43_clm)
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
        let (net_stack, net_runner) = net::new(net_driver, config, resources, seed);
        (net_stack, net_runner)
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
    const USE_TLS: bool = false;
    let client_state = {
        const N: usize = 1;
        const TX_SZ: usize = 4096;
        const RX_SZ: usize = 4096;
        static STATIC_CELL: StaticCell<net::tcp::client::TcpClientState<N, TX_SZ, RX_SZ>> = StaticCell::new();
        STATIC_CELL.init(net::tcp::client::TcpClientState::<N, TX_SZ, RX_SZ>::new())
    };
    let buf_response = {
        const SIZE: usize = 4096;
        static STATIC_CELL: StaticCell<[u8; SIZE]> = StaticCell::new();
        STATIC_CELL.init([0; SIZE])
    };
    loop {
        let tcp_client = net::tcp::client::TcpClient::new(net_stack, client_state);
        let dns_socket = net::dns::DnsSocket::new(net_stack);
        let (mut http_client, url) = if USE_TLS {
            let tls_read_buffer = {
                static STATIC_CELL: StaticCell<[u8; 16640]> = StaticCell::new();
                STATIC_CELL.init([0u8; 16640])
            };
            let tls_write_buffer = {
                static STATIC_CELL: StaticCell<[u8; 16640]> = StaticCell::new();
                STATIC_CELL.init([0u8; 16640])
            };
            let tls_config = {
                let mut rng = rp::clocks::RoscRng;
                let seed = rng.next_u64();
                reqwless::client::TlsConfig::new(seed, tls_read_buffer, tls_write_buffer, reqwless::client::TlsVerify::None)
            };
            let http_client = reqwless::client::HttpClient::new_with_tls(&tcp_client, &dns_socket, tls_config);
            let url = "https://httpbin.org/json";
            (http_client, url)
        } else {
            let http_client = reqwless::client::HttpClient::new(&tcp_client, &dns_socket);
            let url = "http://httpbin.org/json";
            (http_client, url)
        };
        info!("connecting to {}", &url);
        let mut request = match http_client.request(reqwless::request::Method::GET, url).await {
            Ok(request) => request,
            Err(e) => {
                error!("Failed to make HTTP request: {:?}", e);
                Timer::after(Duration::from_secs(5)).await;
                continue;
            }
        };
        let response = match request.send(buf_response).await {
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
        info!("Response body length: {} bytes", body_bytes.len());
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
        match serde_json_core::from_slice::<HttpBinResponse>(body_bytes) {
            Ok((output, _used)) => {
                info!("Successfully parsed JSON response!");
                info!("Slideshow title: {:?}", output.slideshow.title);
                info!("Slideshow author: {:?}", output.slideshow.author);
            }
            Err(e) => {
                error!("Failed to parse JSON response: {}", Debug2Format(&e));
            }
        }
        Timer::after(Duration::from_secs(5)).await;
    }
}
