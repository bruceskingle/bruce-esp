use std::{net::{IpAddr, UdpSocket}, sync::{Arc, Mutex}, thread};

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
    match run() {
        Ok(()) => log::info!("Application finished successfully"),
        Err(e) => {
            log::error!("Application failed with error: {}", e);
            panic!("App failed");
        },
    }
}

fn run() -> anyhow::Result<()> {
    let mut features = Vec::<Box<dyn Feature>>::new();
    features.push(Box::new(DynDns2::new()));

    log::info!("Trace 1");
    let mut sparko_cyd = SparkoCyd::new(features)?;
    
    log::info!("Trace 2");
    sparko_cyd.start()?;


    log::info!("Trace 3");
    let current_dns = resolve_local_dns()?;
    info!("Current DNS resolution for home.skingle.org: {}", current_dns);

    let addr = Arc::new(Mutex::new(current_dns));

    // let handler_addr = addr.clone();

    let mut cnt = 0;

    let mut r = 64;
    let mut g = 0;
    let mut b = 0;
    loop {
        log::info!("Top of loop");

        // sparko_cyd.led_manager.set_color(r,g,b)?;

        // let c = r;
        // r = b;
        // b = g;
        // g = c;

        if cnt < 3 {
            match get_public_ip_address() {
                Ok(public_ip) => {
                    cnt = cnt + 1;
                    if public_ip != *addr.clone().lock().unwrap() {
                        log::info!("Public IP changed: {} -> {}", *addr.lock().unwrap(), public_ip);
                        // *addr.lock()? = public_ip;
                    } else {
                        log::info!("Public IP unchanged: {}", public_ip);
                    }
                },
                Err(e) => {
                    log::error!("Failed to get public IP address: {}", e);
                }
            }
        }

        

        // let mut led = led.lock()?;
        // led.toggle()?;
        std::thread::sleep(std::time::Duration::from_secs(10));
    }

    // let peripherals = Peripherals::take()?;
    // let sys_loop = EspSystemEventLoop::take()?;
    // let timer_service = EspTaskTimerService::new()?;

    // let nvs_partition: esp_idf_svc::nvs::EspNvsPartition<esp_idf_svc::nvs::NvsDefault> = EspDefaultNvsPartition::take()?;



    // let mut wifi_manager = //wifi::wifi(peripherals.modem, sys_loop,Some(nvs_partition.clone()),timer_service)?;
    //     WiFiManager::new(peripherals.modem, sys_loop, nvs_partition.clone(),timer_service)?;

    // // let led_red_pin = PinDriver::output(peripherals.pins.gpio4)?;
    // // let led_green_pin = PinDriver::output(peripherals.pins.gpio16)?;
    // // let led_blue_pin = PinDriver::output(peripherals.pins.gpio17)?;

    // let led_manager = LedManager::new(peripherals.ledc.timer0, 
    //     peripherals.ledc.channel0, peripherals.pins.gpio4, 
    //     peripherals.ledc.channel1, peripherals.pins.gpio16,
    //     peripherals.ledc.channel2, peripherals.pins.gpio17)?;
    
    // led_manager.set_color(255, 255, 0)?;

    // // let led_timer: esp_idf_hal::ledc::TIMER0<'_> = peripherals.ledc.timer0;
    // // let led_timer_driver = esp_idf_hal::ledc::LedcTimerDriver::new(led_timer, &esp_idf_hal::ledc::config::TimerConfig::new().frequency(1000.Hz()))?;
 
    // // let led_channel_red = Arc::new(Mutex::new(LedcDriver::new(peripherals.ledc.channel0, &led_timer_driver, peripherals.pins.gpio4)?));
    // // let led_channel_green = Arc::new(Mutex::new(LedcDriver::new(peripherals.ledc.channel1, &led_timer_driver, peripherals.pins.gpio16)?));
    // // let led_channel_blue = Arc::new(Mutex::new(LedcDriver::new(peripherals.ledc.channel2, &led_timer_driver, peripherals.pins.gpio17)?));
    // // let led = Arc::new(Mutex::new(led_pin));

    // let config_manager = ConfigManager::new(nvs_partition)?;
    // let mut server_manager = HttpServerManager::new()?;
    
    // ConfigManager::create_pages(&config_manager, &mut server_manager)?;


    // if config_manager.is_config_valid() {
    //     log::info!("Loaded config");

    //     // start wifi
    //     futures::executor::block_on(
    //         wifi_manager.start_client(&config_manager))?;
    //     info!("Wifi started");

    //     // server_manager.fn_handler("/", esp_idf_svc::http::Method::Get, move |req|  -> anyhow::Result<()> {
    //     //         let mut response = req.into_ok_response()?;
    //     //         // unwrapping the mutex lock calls because if there is a poisoned mutex we want to panic anyway
    //     //         response.write(format!("Hello").as_bytes())?;
    //     //         response.flush()?;
    //     //         led.lock().unwrap().toggle()?;
    //     //         Ok(())
    //     //     })?;
            
    //     let current_dns = resolve_local_dns()?;

    //     info!("Current DNS resolution for home.skingle.org: {}", current_dns);

    //     let addr = Arc::new(Mutex::new(current_dns));

    //     let handler_addr = addr.clone();

    //     // server_manager.fn_handler("/", esp_idf_svc::http::Method::Get, move |req|  -> anyhow::Result<()> {
    //     //     let mut response = req.into_ok_response()?;
    //     //     // unwrapping the mutex lock calls because if there is a poisoned mutex we want to panic anyway
    //     //     response.write(format!("External IP Address is: {}", handler_addr.lock().unwrap()).as_bytes())?;
    //     //     led.lock().unwrap().toggle()?;
    //     //     Ok(())
    //     // })?;

    //     let mut cnt = 0;

    //     let mut r = 0;
    //     let mut g = 255;
    //     let mut b = 255;
    //     loop {
    //         log::info!("Top of loop");

    //         led_manager.set_color(r,g,b)?;

    //         let c = r;
    //         r = b;
    //         b = g;
    //         g = c;

    //         if cnt < 3 {
    //             let public_ip = get_public_ip_address()?;

    //             if public_ip != *addr.clone().lock().unwrap() {
    //                 log::info!("Public IP changed: {} -> {}", *addr.lock().unwrap(), public_ip);
    //                 // *addr.lock()? = public_ip;
    //             } else {
    //                 log::info!("Public IP unchanged: {}", public_ip);
    //             }
    //         }

    //         cnt = cnt + 1;

    //         // let mut led = led.lock()?;
    //         // led.toggle()?;
    //         std::thread::sleep(std::time::Duration::from_secs(10));
    //     }
    // }

    // log::info!("No config found");

    // server_manager.init_ap_pages()?;

    // let server_addr = futures::executor::block_on(wifi_manager.start_access_point())?;

    // thread::spawn(move || captive_dns_server(server_addr));
    
    // loop {
    //         log::info!("Top of AP loop");

            

    //         // let mut led = led.lock()?;
    //         // led.toggle()?;
    //         std::thread::sleep(std::time::Duration::from_secs(10));
    //     }
    
    // Ok(())
}
