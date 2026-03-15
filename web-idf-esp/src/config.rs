use esp_idf_svc::http::Method;
use esp_idf_svc::http::server::EspHttpConnection;
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceConfig {
    pub ssid: String,
    pub wifi_password: String,
    pub ddns_hostname: String,
}

// const SSID: &str = env!("WIFI_SSID");
// const PASS: &str = env!("WIFI_PASS");

// impl Default for DeviceConfig {
//     fn default() -> Self {
//         Self {
//             ssid: SSID.into(),
//             wifi_password: PASS.into(),
//             ddns_hostname: "home.skingle.org".into(),
//         }
//     }
// }

use esp_idf_svc::nvs::*;
use crate::http::HttpServerManager;

pub struct ConfigManager {
    nvs: EspNvs<NvsDefault>,
}

impl ConfigManager {
    pub fn new(server_manager: &mut HttpServerManager<'_>, nvs_partition: EspNvsPartition<NvsDefault>) -> anyhow::Result<Arc<Mutex<Self>>> {
        // let default_nvs: EspNvsPartition<NvsDefault> = EspDefaultNvsPartition::take()?;
        let nvs= EspNvs::new(nvs_partition, "config", true)?;

        let config_manager = Arc::new(Mutex::new(Self {
            nvs,
        }));

        {
            let config_manager_clone = Arc::clone(&config_manager);
            server_manager.fn_handler("/config", Method::Post, move |req| -> anyhow::Result<()> {
                let mut cm = config_manager_clone.lock().unwrap();
                cm.handle_config_update(req)
            })?;
        }

        Ok(config_manager)
    }

    pub fn handle_config_update(&mut self, mut req: esp_idf_svc::http::server::Request<&mut EspHttpConnection>) -> anyhow::Result<()> {
        let mut body = Vec::new();
        req.read(&mut body)?;

        let new_config: DeviceConfig = serde_json::from_slice(&body)?;

        self.save_config(&new_config)?;

        let mut resp = req.into_ok_response()?;
        resp.write(b"OK")?;

        Ok(())
    }

    pub fn load_config(&self) -> Option<DeviceConfig> {
        let mut buf = [0u8; 512];
        if let Ok(Some(data)) = self.nvs.get_blob("device_config", &mut buf) {
            match serde_json::from_slice(&data) {
                Ok(config) => Some(config),
                Err(e) => {
                    log::error!("Failed to parse config: {}", e);
                    None
                }
            }
        } else {
            log::error!("Failed to read config");
            None
        }
    }

    pub fn save_config(&mut self, config: &DeviceConfig) -> anyhow::Result<()> {
        let data = serde_json::to_vec(config)?;
        self.nvs.set_blob("device_config", &data)?;
        Ok(())
    }
}
