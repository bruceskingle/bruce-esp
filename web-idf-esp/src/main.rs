use std::{net::{IpAddr, UdpSocket}, sync::{Arc, Mutex}, thread};

use esp_idf_hal::gpio::PinDriver;
use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::peripherals::Peripherals, http::{Method, client::EspHttpConnection, server::EspHttpServer}, nvs::{EspDefaultNvsPartition, EspNvs}, timer::EspTaskTimerService};
use log::info;
use std::str::FromStr;

mod config;
mod wifi;
mod http;
mod portal;

use std::net::{ToSocketAddrs};

use crate::{config::ConfigManager, http::HttpServerManager, wifi::WiFiManager};

fn resolve_single(name: &str) -> anyhow::Result<IpAddr> {
    let addr = (name, 0)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("DNS returned no addresses"))?;

    Ok(addr.ip())
}

fn resolve_local_dns() -> anyhow::Result<IpAddr> {
    resolve_single("home.skingle.org")
}

fn get_public_ip_address() -> anyhow::Result<IpAddr> {
    // HTTP client
    let connection = EspHttpConnection::new(&esp_idf_svc::http::client::Configuration::default())?;
    let mut client = embedded_svc::http::client::Client::wrap(connection);

    let url = "http://svc.joker.com/nic/myip";

    let request = client.request(Method::Get, url, &[])?;
    let mut response = request.submit()?;

    println!("Status: {}", response.status());

    let mut body = [0u8; 512];
    let bytes_read = response.read(&mut body)?;

    let addr_str = core::str::from_utf8(&body[..bytes_read]).unwrap_or("invalid utf8").trim();
    let addr: IpAddr = IpAddr::from_str(addr_str)?;

    println!(
        "Body: {}",
        addr_str
    );
    println!(
        "IP Address: {}",
        addr
    );
    Ok(addr)
}

fn main() {
    // It is necessary to call this function once. Otherwise, some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    // This is the app level fault barrier.
    // For the moment we just unwrap and panic, but in the future we might want to attempt some sort of recovery or restart.
    run().unwrap();
}

fn run() -> anyhow::Result<()> {


    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let timer_service = EspTaskTimerService::new()?;

    let nvs_partition: esp_idf_svc::nvs::EspNvsPartition<esp_idf_svc::nvs::NvsDefault> = EspDefaultNvsPartition::take()?;



    let mut wifi_manager = //wifi::wifi(peripherals.modem, sys_loop,Some(nvs_partition.clone()),timer_service)?;
        WiFiManager::new(peripherals.modem, sys_loop, nvs_partition.clone(),timer_service)?;
    let led_pin = PinDriver::output(peripherals.pins.gpio16)?;

    let led = Arc::new(Mutex::new(led_pin));

    let mut server_manager = HttpServerManager::new()?;
    let config_manager = ConfigManager::new(&mut server_manager, nvs_partition)?;
    let opt_config = config_manager.lock().unwrap().load_config();


    server_manager.fn_handler("/", esp_idf_svc::http::Method::Get, move |req|  -> anyhow::Result<()> {
            let mut response = req.into_ok_response()?;
            // unwrapping the mutex lock calls because if there is a poisoned mutex we want to panic anyway
            response.write(format!("Hello").as_bytes())?;
            response.flush()?;
            led.lock().unwrap().toggle()?;
            Ok(())
        })?;

    if let Some(config) = opt_config {
        log::info!("Loaded config: {:?}", config);

        let current_dns = resolve_local_dns()?;

        let addr = Arc::new(Mutex::new(current_dns));

        let handler_addr = addr.clone();

        // server_manager.fn_handler("/", esp_idf_svc::http::Method::Get, move |req|  -> anyhow::Result<()> {
        //     let mut response = req.into_ok_response()?;
        //     // unwrapping the mutex lock calls because if there is a poisoned mutex we want to panic anyway
        //     response.write(format!("External IP Address is: {}", handler_addr.lock().unwrap()).as_bytes())?;
        //     led.lock().unwrap().toggle()?;
        //     Ok(())
        // })?;

        let mut cnt = 0;

        loop {
            log::info!("Top of loop");

            if cnt < 3 {
                let public_ip = get_public_ip_address()?;

                if public_ip != *addr.clone().lock().unwrap() {
                    log::info!("Public IP changed: {} -> {}", *addr.lock().unwrap(), public_ip);
                    // *addr.lock()? = public_ip;
                } else {
                    log::info!("Public IP unchanged: {}", public_ip);
                }
            }

            cnt = cnt + 1;

            // let mut led = led.lock()?;
            // led.toggle()?;
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    }

    log::info!("No config found");

    server_manager.fn_handler("/generate_204", Method::Get, |req| {
        let mut resp = req.into_response(302, None, &[("Location", "/")])?;
        resp.write(b"")?;
        Ok(())
    })?;

    server_manager.fn_handler("/hotspot-detect.html", Method::Get, |req| {
        let mut resp = req.into_response(302, None, &[("Location", "/")])?;
        resp.write(b"")?;
        Ok(())
    })?;

    let server_addr = futures::executor::block_on(wifi_manager.start_access_point())?;

    thread::spawn(move || captive_dns_server(server_addr));
    
    loop {
            log::info!("Top of AP loop");

            

            // let mut led = led.lock()?;
            // led.toggle()?;
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    
    Ok(())
}

fn captive_dns_server(server_addr: std::net::Ipv4Addr)  {
    info!("DNS server start");
    let socket = UdpSocket::bind("0.0.0.0:53").unwrap();
    let addr_bytes = server_addr.octets();
    loop {
        let mut buf = [0u8; 512];

        info!("DNS server recv_from...");
        let (size, src) = socket.recv_from(&mut buf).unwrap();

        info!("DNS server recv_from...{:?}", &buf[..size]);

        let response = build_dns_response(&buf[..size], &addr_bytes);

        socket.send_to(&response, src).unwrap();
    }
}

fn build_dns_response(query: &[u8], server_addr: &[u8; 4]) -> Vec<u8> {
    info!("Received DNS query: {:?}", query);
    let mut resp = query.to_vec();

    resp[2] |= 0x80; // set QR bit (response)
    resp[3] |= 0x80; // set RD bit (recursion desired, optional)

    // Set ANCOUNT to 1 (answer count)
    resp[6] = 0x00;
    resp[7] = 0x01;

    resp.extend_from_slice(&[
        0xc0, 0x0c, // pointer to domain
        0x00, 0x01, // type A
        0x00, 0x01, // class IN
        0x00, 0x00, 0x00, 0x3c, // TTL (60 seconds)
        0x00, 0x04, // data length (4 bytes for IPv4)

        server_addr[0], server_addr[1], server_addr[2], server_addr[3] // IP address
    ]);

    info!("Sending DNS response: {:?}", resp);
    
    resp
}
