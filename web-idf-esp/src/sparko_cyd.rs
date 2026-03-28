use std::{backtrace::Backtrace, net::{IpAddr, UdpSocket}, sync::{Arc, Mutex}, thread};

use esp_idf_hal::{gpio::PinDriver, ledc::LedcDriver};
use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::peripherals::Peripherals, http::{Method, client::EspHttpConnection, server::EspHttpServer}, nvs::{EspDefaultNvsPartition, EspNvs}, timer::EspTaskTimerService};
use log::info;
use std::str::FromStr;
use esp_idf_svc::sntp::*;
use chrono::{DateTime, Local};
use std::time::SystemTime;

use crate::{Feature, config::ConfigManager, http::HttpServerManager, led::LedManager, wifi::WiFiManager};


use esp_idf_sys::*;
use std::ffi::CStr;

fn list_nvs_keys() {
    info!("Listing NVS keys:");
    unsafe {
    let mut it: nvs_iterator_t = std::ptr::null_mut();
    let part = CStr::from_bytes_with_nul_unchecked(b"nvs\0");
   

    let res = nvs_entry_find(
        part.as_ptr(), // partition name 
        // std::ptr::null(), // partition
        std::ptr::null(), // namespace
        nvs_type_t_NVS_TYPE_ANY,
        &mut it,
    );

    if res == ESP_OK {
        info!("NVS keys found:");
        while !it.is_null() {
            let mut info: nvs_entry_info_t = std::mem::zeroed();

            nvs_entry_info(it, &mut info);

            

            let namespace = CStr::from_ptr(info.namespace_name.as_ptr())
                .to_str()
                .unwrap();

            let key = CStr::from_ptr(info.key.as_ptr())
                .to_str()
                .unwrap();

            info!("NS: {}, Key: {}", namespace, key);

            nvs_entry_next(&mut it);
        }

        nvs_release_iterator(it);
    }
    else {
        info!("Failed to list NVS keys: {}", res);
    }
    info!("Finished listing NVS keys");
}
}

pub struct SparkoCyd {
    pub wifi_manager: WiFiManager<'static>,
    pub led_manager: LedManager<'static>,
    pub config_manager: Arc<ConfigManager>,
    pub server_manager: HttpServerManager<'static>,
    features: Vec::<Box<dyn Feature>>,
    ap_mode: Arc<Mutex<bool>>,
}



impl SparkoCyd {
    pub fn new(features: Vec::<Box<dyn Feature>>) -> anyhow::Result<Self> {
        // // First validate features
        // // I am doing this iteratively to avoid allocations on the heap. This might be early optimization.......
        // for i in 0..features.len() {
        //     let feature = &features[i];
        //     let config = feature.get_config();

        //     for reserved_name in RESERVED_FEATURE_NAMES.iter() {
        //         if config.name == *reserved_name {
        //             return Err(anyhow::anyhow!("Feature name '{}' is reserved and cannot be used", config.name));
        //         }
        //     }

        //     // config detects this already
        //     // for j in (i+1)..features.len() {
        //     //     let other_feature = &features[j];
        //     //     if config.name == other_feature.get_config().name {
        //     //         return Err(anyhow::anyhow!("Duplicate feature name found: {}", config.name));
        //     //     }
        //     // }
        // }


        let failure_reason: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let ap_mode = Arc::new(Mutex::new(false));

        let peripherals = Peripherals::take()?;


        let led_manager = LedManager::new(peripherals.ledc.timer0, 
            peripherals.ledc.channel0, peripherals.pins.gpio4, 
            peripherals.ledc.channel1, peripherals.pins.gpio16,
            peripherals.ledc.channel2, peripherals.pins.gpio17)?;
        
        led_manager.set_color(64, 64, 0)?;


        let sys_loop = EspSystemEventLoop::take()?;
        let timer_service = EspTaskTimerService::new()?;

        let nvs_partition: esp_idf_svc::nvs::EspNvsPartition<esp_idf_svc::nvs::NvsDefault> = EspDefaultNvsPartition::take()?;

        list_nvs_keys();

        let wifi_manager = //wifi::wifi(peripherals.modem, sys_loop,Some(nvs_partition.clone()),timer_service)?;
            WiFiManager::new(peripherals.modem, sys_loop, nvs_partition.clone(), failure_reason.clone())?;

        // let led_red_pin = PinDriver::output(peripherals.pins.gpio4)?;
        // let led_green_pin = PinDriver::output(peripherals.pins.gpio16)?;
        // let led_blue_pin = PinDriver::output(peripherals.pins.gpio17)?;


        // let led_timer: esp_idf_hal::ledc::TIMER0<'_> = peripherals.ledc.timer0;
        // let led_timer_driver = esp_idf_hal::ledc::LedcTimerDriver::new(led_timer, &esp_idf_hal::ledc::config::TimerConfig::new().frequency(1000.Hz()))?;
    
        // let led_channel_red = Arc::new(Mutex::new(LedcDriver::new(peripherals.ledc.channel0, &led_timer_driver, peripherals.pins.gpio4)?));
        // let led_channel_green = Arc::new(Mutex::new(LedcDriver::new(peripherals.ledc.channel1, &led_timer_driver, peripherals.pins.gpio16)?));
        // let led_channel_blue = Arc::new(Mutex::new(LedcDriver::new(peripherals.ledc.channel2, &led_timer_driver, peripherals.pins.gpio17)?));
        // let led = Arc::new(Mutex::new(led_pin));

        let config_manager = ConfigManager::new(nvs_partition, &features, failure_reason, ap_mode.clone())?;
        let mut server_manager = HttpServerManager::new()?;
        
        ConfigManager::create_pages(&config_manager, &mut server_manager)?;

        Ok(Self {
            wifi_manager,
            led_manager,
            config_manager,
            server_manager,
            features,
            ap_mode,
        })
    }
    

    pub fn start_client(&mut self) -> anyhow::Result<()> {

            // start wifi

            self.wifi_manager.start_client(&self.config_manager)?;
            info!("Wifi started");

            let sntp = EspSntp::new_default()?;
            
            info!("SNTP started, waiting for time sync...");

            loop {
                if let SyncStatus::Completed = sntp.get_sync_status() {
                    break
                }
                info!("still waiting for time sync...");
                std::thread::sleep(std::time::Duration::from_millis(500));
    }

            // std::thread::sleep(std::time::Duration::from_secs(2));
 
            let now = SystemTime::now();
            let datetime: DateTime<Local> = now.into();
            info!("Time synced: {}", datetime.format("%Y-%m-%d %H:%M:%S"));

            self.led_manager.set_color(0, 64, 0)?;

            loop {
                    log::info!("Top of loop");

                    let now = SystemTime::now();
                    let datetime: DateTime<Local> = now.into();
                    info!("Time synced: {}", datetime.format("%Y-%m-%d %H:%M:%S"));

            
                    let heap_free = unsafe { esp_get_free_heap_size() };
                    let heap_min = unsafe { esp_get_minimum_free_heap_size() };
                    log::info!("heap free={} min={}", heap_free, heap_min);
                    
                    // TODO: force a reset if we run low on heap

                    std::thread::sleep(std::time::Duration::from_secs(10));
                }
    }
    

    pub fn start(&mut self) -> anyhow::Result<()> {
        log::info!("sparko_cyd: top of run");
        if self.config_manager.is_core_config_valid() {
            log::info!("Loaded config");

            if let Err(error) = self.start_client() {
                log::error!("Error starting client: {}", error);
                self.led_manager.set_color(64, 0, 0)?;
            }



            // return Ok(());

            // server_manager.fn_handler("/", esp_idf_svc::http::Method::Get, move |req|  -> anyhow::Result<()> {
            //         let mut response = req.into_ok_response()?;
            //         // unwrapping the mutex lock calls because if there is a poisoned mutex we want to panic anyway
            //         response.write(format!("Hello").as_bytes())?;
            //         response.flush()?;
            //         led.lock().unwrap().toggle()?;
            //         Ok(())
            //     })?;
                

            

            // server_manager.fn_handler("/", esp_idf_svc::http::Method::Get, move |req|  -> anyhow::Result<()> {
            //     let mut response = req.into_ok_response()?;
            //     // unwrapping the mutex lock calls because if there is a poisoned mutex we want to panic anyway
            //     response.write(format!("External IP Address is: {}", handler_addr.lock().unwrap()).as_bytes())?;
            //     led.lock().unwrap().toggle()?;
            //     Ok(())
            // })?;

            
        }
        else {
            self.led_manager.set_color(0, 0, 64)?;
            info!("Invalid config, starting AP mode");
        }

        if let Some(reason) = self.config_manager.failure_reason.lock().unwrap().as_ref() {
                info!("APMODE Failure reason present, showing error message on config page: {}", reason);
            }
            else {
                info!("APMODE No failure reason, not showing error message on config page");
            }

        *self.ap_mode.lock().unwrap() = true;
        

        self.server_manager.init_ap_pages()?;

        let server_addr = self.wifi_manager.start_access_point()?;

        thread::spawn(move || Self::captive_dns_server(server_addr));
        
        loop {
            log::info!("Top of AP loop");

            

            // let mut led = led.lock()?;
            // led.toggle()?;
            std::thread::sleep(std::time::Duration::from_secs(10));
        }

        fn system_halt<S: AsRef<str>>(s: S) {
            // TODO: Implement BSOD or similar system halt mechanism here
            println!("{}", s.as_ref());

            let bt = Backtrace::force_capture();
            println!("Stack trace:\n{bt}");

            std::process::exit(1);
        }
    }

    // pub fn main_loop(&mut self) -> anyhow::Result<()> {
    //     loop {
    //         let now = std::time::SystemTime::now();
    //         let datetime: DateTime<Local> = now.into();
    //         info!("Top of main loop time is : {}", datetime.format("%Y-%m-%d %H:%M:%S"));

    //         let wake = (now + Duration::hours(1))
    //             .with_minute(0).unwrap()
    //             .with_second(0).unwrap()
    //             .with_nanosecond(0).unwrap();
            
    //         std::thread::sleep(std::time::Duration::from_secs(10));
    //     }
    // }

    fn captive_dns_server(server_addr: std::net::Ipv4Addr)  {
        info!("DNS server start");
        let socket = UdpSocket::bind("0.0.0.0:53").unwrap();
        let addr_bytes = server_addr.octets();
        loop {
            let mut buf = [0u8; 512];

            // info!("DNS server recv_from...");
            let (size, src) = socket.recv_from(&mut buf).unwrap();

            // info!("DNS server recv_from...{:?}", &buf[..size]);

            let response = Self::build_dns_response(&buf[..size], &addr_bytes);

            socket.send_to(&response, src).unwrap();
        }
    }

    fn build_dns_response(query: &[u8], server_addr: &[u8; 4]) -> Vec<u8> {
        // info!("Received DNS query: {:?}", query);
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

        // info!("Sending DNS response: {:?}", resp);
        
        resp
    }
}