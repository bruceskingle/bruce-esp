use std::{net::{IpAddr, UdpSocket}, sync::{Arc, Mutex}, thread, time::SystemTime};

use chrono::{DateTime, Local};
use esp_idf_hal::{gpio::PinDriver, ledc::LedcDriver};
use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::peripherals::Peripherals, http::{Method, client::EspHttpConnection, server::EspHttpServer}, nvs::{EspDefaultNvsPartition, EspNvs}, timer::EspTaskTimerService};
use log::info;
use web_idf_esp::sparko_cyd::SparkoCyd;
use std::str::FromStr;



use std::net::{ToSocketAddrs};

use web_idf_esp::Feature;
use web_idf_esp::dyndns2::DynDns2;

// use crate::{config::ConfigManager, http::HttpServerManager, led::LedManager, wifi::WiFiManager};

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
    // let connection = EspHttpConnection::new(&esp_idf_svc::http::client::Configuration::default())?;
    // let mut client = embedded_svc::http::client::Client::wrap(connection);

    let mut client = embedded_svc::http::client::Client::wrap(EspHttpConnection::new(&esp_idf_svc::http::client::Configuration {
        // use_global_ca_store: true,
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    })?);

    let url = "https://svc.joker.com/nic/myip";

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
    match run() {
        Ok(()) => log::info!("Application finished successfully"),
        Err(e) => {
            log::error!("Application failed with error: {}", e);
            panic!("App failed");
        },
    }
}

fn run() -> anyhow::Result<()> {
    // let mut builder = SparkoCyd::Builder::new();

    // let mut sparko_cyd = builder
    //     .with_feature(DynDns2::new())
    //     .build()?;

    let mut features = Vec::<Box<dyn Feature>>::new();
    features.push(Box::new(DynDns2::new()));

    log::info!("Trace 1");
    let mut sparko_cyd = SparkoCyd::new(features)?;

    let cloned_ap_mode = sparko_cyd.ap_mode.clone();
    sparko_cyd.server_manager.fn_handler("/", Method::Get, move |req| {

            // info!("Received request for / from {}", req.connection().remote_addr());

            info!("Received {:?} request for {}", req.method(), req.uri());

            if cloned_ap_mode.lock().unwrap().clone() {
                let mut resp = req.into_response(
                    302,
                    Some("Found"),
                    &[("Location", "/new-path")],
                )?;
            }
            else {

                let mut resp = req.into_ok_response()?;
                resp.write(r#"
                    <!DOCTYPE html>
                    <html lang="en">
                    <head>
                        <meta charset="utf-8" />
                        <meta name="viewport" content="width=device-width, initial-scale=1" />
                        <title>ESP32 Home</title>
                        <link rel="stylesheet" href="/main.css">
                    </head>
                    <body>
                        <div class="page">
                            <h1>ESP32 Home</h1>
                            <p>Welcome to the ESP32 home page!</p>
                            <p>Current time: "#.as_bytes())?;

                let now = Local::now();
                let time = now.format("%Y-%m-%d %H:%M:%S").to_string();
                resp.write(time.as_bytes())?;
                resp.write(r#"</p>
                        </div>
                    </body>
                    </html>
                    "#.as_bytes())?;
            }
            Ok(())
        })?;

    
    log::info!("Trace 2");
    sparko_cyd.start()
    // ?;
    // sparko_cyd.run()
}


    // log::info!("Trace 3");
    // let current_dns = resolve_local_dns()?;
    // info!("Current DNS resolution for home.skingle.org: {}", current_dns);

    // let addr = Arc::new(Mutex::new(current_dns));

    // // let handler_addr = addr.clone();

    // let mut cnt = 0;

    // let mut r = 64;
    // let mut g = 0;
    // let mut b = 0;
    // loop {
    //     log::info!("Top of loop");

    //     // sparko_cyd.led_manager.set_color(r,g,b)?;

    //     // let c = r;
    //     // r = b;
    //     // b = g;
    //     // g = c;

    //     if cnt < 3 {
    //         match get_public_ip_address() {
    //             Ok(public_ip) => {
    //                 cnt = cnt + 1;
    //                 if public_ip != *addr.clone().lock().unwrap() {
    //                     log::info!("Public IP changed: {} -> {}", *addr.lock().unwrap(), public_ip);
    //                     // *addr.lock()? = public_ip;
    //                 } else {
    //                     log::info!("Public IP unchanged: {}", public_ip);
    //                 }
    //             },
    //             Err(e) => {
    //                 log::error!("Failed to get public IP address: {}", e);
    //             }
    //         }
    //     }

        

    //     // let mut led = led.lock()?;
    //     // led.toggle()?;
    //     std::thread::sleep(std::time::Duration::from_secs(10));
    // }


