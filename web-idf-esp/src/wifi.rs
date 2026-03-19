// use embedded_svc::wifi::{Configuration, AuthMethod};
use embedded_svc::wifi::ClientConfiguration;
use esp_idf_hal::modem::WifiModemPeripheral;
use esp_idf_svc::wifi::AccessPointConfiguration;
use esp_idf_svc::wifi::AsyncWifi;
use esp_idf_svc::wifi::AuthMethod;
use esp_idf_svc::wifi::Configuration;
use esp_idf_svc::wifi::EspWifi;
use web_idf_esp::PASSWORD_LEN;
use web_idf_esp::SSID_LEN;
use std::net::Ipv4Addr;
use std::sync::Arc;
use log::info;
use esp_idf_svc::handle::RawHandle;
use esp_idf_svc::timer::{EspTimerService, Task};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::ping::EspPing;
use esp_idf_svc::nvs::EspNvsPartition;
use esp_idf_svc::nvs::NvsDefault;

use crate::config::ConfigManager;

pub struct WiFiManager<'a> {
    wifi: AsyncWifi<EspWifi<'a>>,
}

impl WiFiManager<'_> {
    pub fn new(
        modem: impl WifiModemPeripheral + 'static,
        sysloop: EspSystemEventLoop,
        nvs: EspNvsPartition<NvsDefault>,
        timer_service: EspTimerService<Task>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            wifi: AsyncWifi::wrap(
                EspWifi::new(modem, sysloop.clone(), Some(nvs))?,
                sysloop,
                timer_service.clone(),
            )?
        })
    }

    pub async fn start_access_point(&mut self) -> anyhow::Result<std::net::Ipv4Addr> {
        

        let ap_config = AccessPointConfiguration {
            ssid: heapless::String::<SSID_LEN>::try_from("ESP32-Setup").unwrap(),
            password: heapless::String::<PASSWORD_LEN>::try_from("password").unwrap(),
            channel: 1,
            auth_method: AuthMethod::WPA2Personal,
            max_connections: 4,
            ..Default::default()
        };

        info!("Starting WiFi Access Point with config: {:?}", ap_config);

        self.wifi.set_configuration(&Configuration::AccessPoint(ap_config))?;

        // self.wifi.start().await?;

        let ip_info = self.wifi.wifi().ap_netif().get_ip_info()?;
        info!("WiFi Access Point IP Info: {:?}", ip_info);

        // Start the AP first, then update the DHCP server DNS option.
        self.wifi.start().await?;

        // Desired DNS server for clients.
        let dns_server = ip_info.ip; //Ipv4Addr::new(1, 1, 1, 1);

        let dns_info = esp_idf_sys::esp_netif_dns_info_t {
            ip: esp_idf_sys::esp_ip_addr_t {
                type_: esp_idf_sys::lwip_ip_addr_type_IPADDR_TYPE_V4 as u8,
                u_addr: esp_idf_sys::_ip_addr__bindgen_ty_1 {
                    ip4: esp_idf_sys::esp_ip4_addr_t {
                        addr: u32::from_le_bytes(dns_server.octets()),
                    },
                },
            },
        };

        unsafe {
            let netif = self.wifi.wifi().ap_netif();
            let handle = netif.handle();

            let res_stop = esp_idf_sys::esp_netif_dhcps_stop(handle);
            if res_stop != esp_idf_sys::ESP_OK {
                log::warn!("esp_netif_dhcps_stop failed: {:?}", res_stop);
            }

            let res_set = esp_idf_sys::esp_netif_set_dns_info(
                handle,
                esp_idf_sys::esp_netif_dns_type_t_ESP_NETIF_DNS_MAIN,
                &dns_info as *const _ as *mut _,
            );
            if res_set != esp_idf_sys::ESP_OK {
                log::warn!("esp_netif_set_dns_info failed: {:?}", res_set);
            }

            let url_str = format!("http://{}/\0", ip_info.ip);
            let url = url_str.as_bytes();
            // let url = b"http://192.168.4.1/\0";

            esp_idf_sys::esp_netif_dhcps_option(
                handle,
                esp_idf_sys::esp_netif_dhcp_option_mode_t_ESP_NETIF_OP_SET,
                114,
                url.as_ptr() as *mut _,
                url.len() as u32
            );

            let res_start = esp_idf_sys::esp_netif_dhcps_start(handle);
            if res_start != esp_idf_sys::ESP_OK {
                log::warn!("esp_netif_dhcps_start failed: {:?}", res_start);
            }

            // Verify the DNS was set correctly
            let mut dns_out = esp_idf_sys::esp_netif_dns_info_t {
                ip: esp_idf_sys::esp_ip_addr_t {
                    type_: 0,
                    u_addr: esp_idf_sys::_ip_addr__bindgen_ty_1 {
                        ip4: esp_idf_sys::esp_ip4_addr_t { addr: 0 }
                    }
                }
            };
            let res_get = esp_idf_sys::esp_netif_get_dns_info(handle, esp_idf_sys::esp_netif_dns_type_t_ESP_NETIF_DNS_MAIN, &mut dns_out);
            if res_get == esp_idf_sys::ESP_OK {
                let addr_net = dns_out.ip.u_addr.ip4.addr;
                let octets = addr_net.to_be_bytes(); // network to host
                let retrieved_dns = Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]);
                log::info!("Retrieved DNS server: {:?}", retrieved_dns);
            } else {
                log::warn!("esp_netif_get_dns_info failed: {:?}", res_get);
            }
        }

        let ip_info = self.wifi.wifi().ap_netif().get_ip_info()?;
        info!("WiFi Access Point IP Info after dns config: {:?}", ip_info);
// let mut dns = dns_info_from_ipv4(ip_info.ip);

//         unsafe {

// //             use esp_idf_svc::ipv4::Ipv4AddrExt;

// //             let mut dns_info = esp_netif_dns_info_t {
// //                 ip: ip_info.ip.into(),
// // };


//             // use esp_idf_sys::*;
//             // use esp_idf_svc::handle::RawHandle;

//             // let netif = self.wifi.wifi().ap_netif();

//             // esp_netif_dhcps_stop(netif.handle());

//             // let octets = ip_info.ip.octets();
//             // let addr = u32::from_be_bytes(octets);


//             // let mut dns_info = esp_netif_dns_info_t {
//             //     ip: esp_ip_addr_t {
//             //         type_: esp_ip_addr_type_t_ESP_IPADDR_TYPE_V4,
//             //         u_addr: esp_ip_addr__bindgen_ty_1 {
//             //             ip4: esp_ip4_addr_t { addr },
//             //         },
//             //     },
//             // };

//             esp_idf_sys::esp_netif_set_dns_info(
//                 netif.handle(),
//                 esp_idf_sys::esp_netif_dns_type_t::ESP_NETIF_DNS_MAIN,
//                 dns_info,
//             );
//         }

        Ok(ip_info.ip)
    }

    pub async fn start_client(&mut self, config_manager: &Arc<ConfigManager>) -> anyhow::Result<std::net::Ipv4Addr> {

        let wifi_configuration: embedded_svc::wifi::Configuration = embedded_svc::wifi::Configuration::Client(ClientConfiguration {
            ssid: heapless::String::<32>::try_from(config_manager.get_valid_config(crate::config::SSID)?.as_str()).unwrap(),
            bssid: None,
            auth_method: embedded_svc::wifi::AuthMethod::WPA2Personal,
            password: heapless::String::<64>::try_from(config_manager.get_valid_config(crate::config::WIFI_PASSWORD)?.as_str()).unwrap(),
            channel: None,
            scan_method: esp_idf_svc::wifi::ScanMethod::FastScan,
            pmf_cfg: esp_idf_svc::wifi::PmfConfiguration::NotCapable,
        });

        self.wifi.set_configuration(&wifi_configuration)?;

        self.wifi.start().await?;
        info!("Wifi started");

        self.wifi.connect().await?;
        info!("Wifi connected");

        self.wifi.wait_netif_up().await?;
        info!("Wifi netif up");

        let ip_info = self.wifi.wifi().sta_netif().get_ip_info()?;

        println!("Wifi DHCP info: {:?}", ip_info);
        
        EspPing::default().ping(ip_info.subnet.gateway, &esp_idf_svc::ping::Configuration::default())?;

        Ok(ip_info.ip)
    }

    // pub fn connect(&self) -> Result<AsyncWifi<EspWifi<'static>>> {
    //     wifi(self.modem, self.sysloop.clone(), self.nvs.clone(), self.timer_service.clone())
    // }
}

// fn dns_info_from_ipv4(ip: Ipv4Addr) -> esp_netif_dns_info_t {
//     let addr = u32::from_be_bytes(ip.octets());

//     esp_netif_dns_info_t {
//         ip: esp_idf_sys::esp_ip_addr_t {
//             type_: esp_idf_sys::esp_ip_addr_type_t::ESP_IPADDR_TYPE_V4,
//             u_addr: esp_idf_sys::esp_ip_addr__bindgen_ty_1 {
//                 ip4: esp_idf_sys::esp_ip4_addr_t { addr },
//             },
//         },
//     }
// }

// pub fn wifi<'a>(
//     modem: impl WifiModemPeripheral + 'static,
//     sysloop: EspSystemEventLoop,
//     nvs: Option<EspNvsPartition<NvsDefault>>,
//     timer_service: EspTimerService<Task>,
// ) -> Result<AsyncWifi<EspWifi<'static>>> {
//     use futures::executor::block_on;

//     let mut wifi = AsyncWifi::wrap(
//         EspWifi::new(modem, sysloop.clone(), nvs)?,
//         sysloop,
//         timer_service.clone(),
//     )?;

//     block_on(connect_wifi(&mut wifi))?;

//     let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

//     println!("Wifi DHCP info: {:?}", ip_info);
    
//     EspPing::default().ping(ip_info.subnet.gateway, &esp_idf_svc::ping::Configuration::default())?;
//     Ok(wifi)

// }

// async fn connect_wifi(wifi: &mut AsyncWifi<EspWifi<'static>>) -> anyhow::Result<()> {
//     let wifi_configuration: embedded_svc::wifi::Configuration = embedded_svc::wifi::Configuration::Client(ClientConfiguration {
//         ssid: heapless::String::<32>::try_from(SSID).unwrap(),
//         bssid: None,
//         auth_method: embedded_svc::wifi::AuthMethod::WPA2Personal,
//         password: heapless::String::<64>::try_from(PASS).unwrap(),
//         channel: None,
//         scan_method: esp_idf_svc::wifi::ScanMethod::FastScan,
//         pmf_cfg: esp_idf_svc::wifi::PmfConfiguration::NotCapable,
//     });

//     wifi.set_configuration(&wifi_configuration)?;

//     wifi.start().await?;
//     info!("Wifi started");

//     wifi.connect().await?;
//     info!("Wifi connected");

//     wifi.wait_netif_up().await?;
//     info!("Wifi netif up");

//     Ok(())
// }