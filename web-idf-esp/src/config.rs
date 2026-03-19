use esp_idf_svc::http::Method;
use esp_idf_svc::http::server::EspHttpConnection;

use indexmap::IndexMap;
use log::info;
use serde::{Serialize, Deserialize};
use url::form_urlencoded;
use web_idf_esp::{PASSWORD_LEN, SSID_LEN};
use std::{sync::{Arc, Mutex}};

use anyhow::anyhow;

#[derive(Debug)]
pub enum TypedValue {
    String(usize, Option<String>),
    Int32(Option<i32>),
    Int64(Option<i64>)
}

impl TypedValue {
    pub fn is_type_compatible(&self, other: &TypedValue) -> bool {
        match self {
            TypedValue::String(len, _value) => {
                if let TypedValue::String(other_len, _) = other {
                    return len == other_len;
                }
                false
            },
            TypedValue::Int32(_) => matches!(other, TypedValue::Int32(_)),
            TypedValue::Int64(_) => matches!(other, TypedValue::Int64(_) ),
        }
    }

    pub fn is_none(&self) -> bool {
        match self {
            TypedValue::String(_len, val) => val.is_none(),
            TypedValue::Int32(val) => val.is_none(),
            TypedValue::Int64(val) => val.is_none(),
        }
    }
    
    fn read_from_nvs(&self, nvs: &EspNvs<NvsDefault>, name: &str) -> TypedValue {
        info!("Reading config value {} from NVS", name);
        let result = match self {
            TypedValue::String(len, _) => {
                let mut buf = vec![0u8; *len as usize];

                let x = nvs.get_str(name, buf.as_mut_slice());
                match x {
                    Ok(str) => log::info!("Read string value for {} from NVS: {:?}", name, str),
                    Err(e) => log::info!("No string value for {} in NVS: {:?}", name, e),
                }

                if let Some(str)= nvs.get_str(name, buf.as_mut_slice()).ok().flatten() {
                    TypedValue::String(*len, Some(str.to_string()))
                } else {
                    TypedValue::String(*len, None)
                }
            },
            TypedValue::Int32(_) => TypedValue::Int32(nvs.get_i32(name).ok().flatten()),
            TypedValue::Int64(_) => TypedValue::Int64(nvs.get_i64(name).ok().flatten()),
        };
        info!("Finished reading config value {} from NVS: {:?}", name, result);
        result
    }
    
    fn to_string(&self) -> String {
        match self {
            TypedValue::String(_len, Some(val)) => val.clone(),
            TypedValue::Int32(Some(val)) => val.to_string(),
            TypedValue::Int64(Some(val)) => val.to_string(),
            _ => "".to_string(),
        }
    }

    fn to_none(&self) -> Self {
        match self {
            TypedValue::String(len, val) => TypedValue::String(*len, None),
            TypedValue::Int32(val) => TypedValue::Int32(None),
            TypedValue::Int64(val) => TypedValue::Int64(None),
        }
    }
    
    fn from_str(&self, str_val: &str) -> anyhow::Result<TypedValue> {
        Ok(match self {

            TypedValue::String(len, _) => {
                if str_val.len() > *len as usize {
                    anyhow::bail!("String value too long: max length is {}", len);
                } else {
                    TypedValue::String(*len, Some(str_val.to_string()))
                    
                }
                
            },
            TypedValue::Int32(_) => TypedValue::Int32(Some(str_val.parse::<i32>()?)),
            TypedValue::Int64(_) => TypedValue::Int64(Some(str_val.parse::<i64>()?)),
        })
    }
}


#[derive(Debug)]
pub struct ConfigValue {
    value: TypedValue,
    required: bool,
}

impl ConfigValue {
    fn read_from_nvs(&mut self, nvs: &EspNvs<NvsDefault>, name: &str) {
        let nv = self.value.read_from_nvs(nvs, name);
        self.value = nv;
    }
}


type DeviceConfig = IndexMap<String, ConfigValue>;

// #[derive(Serialize, Deserialize, Debug)]
// pub struct DeviceConfig {
//     pub ssid: String,
//     pub wifi_password: String,
//     pub ddns_hostname: String,
// }

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
    config_impl: Mutex<ConfigManagerImpl>,
}

impl ConfigManager {
    pub fn new(nvs_partition: EspNvsPartition<NvsDefault>) -> anyhow::Result<Arc<ConfigManager>> {
        Ok(Arc::new(ConfigManager {
            config_impl: Mutex::new(ConfigManagerImpl::new(nvs_partition)?),
        }))
    }

    pub fn get_valid_config<'a>(&'a self, key: &str) -> anyhow::Result<String> {
        if let Some(value) = self.config_impl.lock().unwrap().device_config.get(key) {
            Ok(value.value.to_string())
        }
        else {
            Err(anyhow!("Config value {} is missing", key))
        }
    }

    pub fn is_config_valid(&self) -> bool {
        self.config_impl.lock().unwrap().is_config_valid()
    }

    pub fn create_pages(config_manager: &Arc<Self>, server_manager: &mut HttpServerManager<'_>) -> anyhow::Result<()> {

        // {
        //     let config_manager_clone = Arc::clone(&config_manager);
        //     server_manager.fn_handler("/config", Method::Post, move |req| -> anyhow::Result<()> {
        //         let mut cm = config_manager_clone.config_impl.lock().unwrap();
        //         cm.handle_config_update(req)
        //     })?;
        // }

        let config_manager_clone = config_manager.clone();

        server_manager.fn_handler("/config", Method::Get, move |req| {

            // info!("Received request for / from {}", req.connection().remote_addr());

            info!("Received {:?} request for {}", req.method(), req.uri());

            let mut resp = req.into_ok_response()?;
            resp.write(r#"
                <!DOCTYPE html>
                <html lang="en">
                <head>
                    <meta charset="utf-8" />
                    <meta name="viewport" content="width=device-width, initial-scale=1" />
                    <title>ESP32 Setup</title>
                    <style>
                        body { font-family: system-ui, -apple-system, BlinkMacSystemFont, sans-serif; margin: 0; padding: 0; background: #f7f7f7; }
                        .page { max-width: 480px; margin: 0 auto; padding: 18px; }
                        h1 { font-size: 1.5rem; margin-bottom: 1rem; }
                        label { display: block; margin: 12px 0 6px; font-weight: 600; }
                        input { width: 100%; padding: 10px 10px; border: 1px solid #ccc; border-radius: 8px; box-sizing: border-box; }
                        button { margin-top: 18px; width: 100%; padding: 12px; font-size: 1rem; border-radius: 10px; border: none; background: #007aff; color: #fff; }
                        button:active { background: #005bb5; }
                    </style>
                </head>
                <body>
                    <div class="page">
                        <h1>ESP32 Setup</h1>
                        <form method="POST" action="/update_config">"#.as_bytes())?;


            for (name, config_value) in &config_manager_clone.config_impl.lock().unwrap().device_config {
                let input_type_buf: String;
                let input_type = match config_value.value {
                    TypedValue::String(len, _) => {
                        input_type_buf = format!("text\" maxlength=\"{}", len);
                        &input_type_buf
                    },
                    TypedValue::Int32(_) | TypedValue::Int64(_) => "number",
                };
                let value = config_value.value.to_string();
                resp.write(format!(r#"
                            <label for="{name}">{name}</label>
                            <input id="{name}" name="{name}" type="{input_type}" autocomplete="off" required value="{value}">
                "#).as_bytes())?;
            }
            resp.write(format!(r#"<button type="submit">Save</button>
                        </form>
                    </div>
                </body>
                </html>
                "#).as_bytes())?;
            Ok(())
        })?;

        let config_manager_clone = config_manager.clone();

        server_manager.fn_handler("/update_config", Method::Post, move |mut req| {

            // info!("Received request for /connect from {}", req.connection().remote_addr());

            info!("Received {:?} request for {}", req.method(), req.uri());
            

            let mut body = Vec::new();
            let mut buf = [0u8; 256];

            loop {
                let read = req.read(&mut buf)?;
                if read == 0 {
                    break;
                }
                body.extend_from_slice(&buf[..read]);
            }

            let form = form_urlencoded::parse(&body)
                .into_owned()
                .collect::<IndexMap<String, String>>();
            // let cm = config_manager_clone.config_impl.lock().unwrap().device_config.iter_mut();
            // let mut device_config = cm.device_config;

            config_manager_clone.config_impl.lock().unwrap().handle_config_form(form);
            // let mut unlocked_config_manager = config_manager_clone.config_impl.lock().unwrap();
            // let nvs = &unlocked_config_manager.nvs;

            // for (name, config_value) in unlocked_config_manager.device_config.iter_mut() {
            //     let str_val = form.get(name).map(|s| s.as_str()).unwrap_or("").trim();
            //     if str_val.len() == 0 {
            //         if config_value.required {
            //             log::error!("Missing required config value: {}", name);
            //         }
            //         else {
            //             if ! config_value.value.is_none() {
            //                 log::info!("Setting optional config value {} to None", name);
            //                 config_value.value = config_value.value.to_none();
            //                 nvs.remove(name).ok();
            //             }
            //         }
            //     }
            //     else {
            //         match config_value.value.from_str(str_val) {
            //             Ok(new_value) => {
            //                 config_value.value = new_value;
            //             }
            //             Err(e) => {
            //                 log::error!("Failed to parse config value for {}: {}", name, e);
            //             }
            //         }
            //     }
            // }


            let mut resp = req.into_ok_response()?;
            resp.write(b"Saved!. Rebooting...(NOT)")?;

            // std::thread::spawn(|| {
            //     std::thread::sleep(std::time::Duration::from_secs(2));
            //     unsafe { esp_idf_sys::esp_restart(); }
            // });

            Ok(())
        })?;

        let config_manager_clone = config_manager.clone();
        server_manager.fn_handler("/generate_204", Method::Get, move |req| {

            let ok = config_manager_clone.config_impl.lock().unwrap().is_config_valid();

            // info!("Received request for /hotspot-detect.html from {}", req.connection().remote_addr());

            info!("Received {:?} request for {} configured={}", req.method(), req.uri(), ok);
            
            
            if ok { 
                let mut resp = req.into_ok_response()?;        
                resp.write(b"<HTML><BODY>Success</BODY></HTML>")?;
            } else {
                let mut resp = req.into_response(302, None, &[("Location", "/config")])?;
                resp.write(b"<HTML><BODY>Not configured</BODY></HTML>")?;
            }
            Ok(())
        })?;

        let config_manager_clone = config_manager.clone();
        server_manager.fn_handler("/hotspot-detect.html", Method::Get, move |req| {

            let ok = config_manager_clone.config_impl.lock().unwrap().is_config_valid();

            // info!("Received request for /hotspot-detect.html from {}", req.connection().remote_addr());

            info!("Received {:?} request for {} configured={} V2", req.method(), req.uri(), ok);
            
            if ok {  
                let mut resp = req.into_ok_response()?;       
                resp.write(b"<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 3.2//EN\">
<HTML>
<HEAD>
	<TITLE>Success</TITLE>
</HEAD>
<BODY>
	Success
</BODY>
</HTML>")?;
            } else {let mut resp = req.into_response(302, None, &[("Location", "/config")])?;
                resp.write(b"<HTML><BODY>Not configured</BODY></HTML>")?;
            }
            Ok(())
        })?;

        let config_manager_clone = config_manager.clone();
        server_manager.fn_handler("/connecttest.txt", Method::Get, move |req| {

            let ok = config_manager_clone.config_impl.lock().unwrap().is_config_valid();

            // info!("Received request for /hotspot-detect.html from {}", req.connection().remote_addr());

            info!("Received {:?} request for {} configured={}", req.method(), req.uri(), ok);
            
            if ok {  
                let mut resp = req.into_ok_response()?;       
                resp.write(b"Microsoft Connect Test")?;
            } else {
                let mut resp = req.into_response(302, None, &[("Location", "/config")])?;
                resp.write(b"Not configured")?;
            }
            Ok(())
        })?;

        Ok(())
    }

    // pub fn handle_config_update(&mut self, mut req: esp_idf_svc::http::server::Request<&mut EspHttpConnection>) -> anyhow::Result<()> {
    //     let mut body = Vec::new();
    //     req.read(&mut body)?;

    //     let new_config: DeviceConfig = serde_json::from_slice(&body)?;

    //     self.save_config(&new_config)?;

    //     let mut resp = req.into_ok_response()?;
    //     resp.write(b"OK")?;

    //     Ok(())
    // }

    // pub fn save_config(&mut self, config: &DeviceConfig) -> anyhow::Result<()> {
    //     let data = serde_json::to_vec(config)?;
    //     self.nvs.set_blob("device_config", &data)?;
    //     Ok(())
    // }
}

struct ConfigManagerImpl {
    nvs: EspNvs<NvsDefault>,
    device_config: DeviceConfig,
}

pub const SSID: &str = "ssid";
pub const WIFI_PASSWORD: &str = "wifi_password";
pub const MDNS_HOSTNAME: &str = "mdns_hostname";

impl ConfigManagerImpl {
    pub fn new(nvs_partition: EspNvsPartition<NvsDefault>) -> anyhow::Result<Self> {
        let mut device_config = IndexMap::new();

        device_config.insert(SSID.to_string(), ConfigValue { value: TypedValue::String(SSID_LEN, None), required: true });
        device_config.insert(WIFI_PASSWORD.to_string(), ConfigValue { value: TypedValue::String(PASSWORD_LEN, None), required: true });
        device_config.insert(MDNS_HOSTNAME.to_string(), ConfigValue { value: TypedValue::String(web_idf_esp::HOSTNAME_LEN, None), required: true });

        let nvs= EspNvs::new(nvs_partition, "config", true)?;

        let mut bare_config_manager = Self {
            nvs,
            device_config,
        };

        bare_config_manager.load_config();
        // let config_manager = Arc::new(Mutex::new(bare_config_manager));


        Ok(bare_config_manager)
    }

    pub fn is_config_valid(&self) -> bool {
        for (name, config_value) in &self.device_config {
            if config_value.required && config_value.value.is_none() {
                log::error!("Missing required config value: {}", name);
                return false;
            }
        }
        info!("Config is valid");
        true
    }

    fn load_config(&mut self) {
        info!("Iterating over NVS items for debugging:");
        let mut keys = self.nvs.keys(None).unwrap();

        loop {
            match keys.next_key() {
                Some((key, data_type)) => log::info!("NVS item: {} of type {:?}", key, data_type),
                None => break,
            }
        }

        info!("Loading config from NVS");
        for (name, config_value) in self.device_config.iter_mut() {
            // let value = TypedValue::read_from_nvs(&self.nvs, name);
            config_value.read_from_nvs(&self.nvs, name);
        }
        info!("Finished loading config: {:?}", self.device_config);
    }

    pub fn handle_config_form(&mut self, form: IndexMap<String, String>) {
        info!("Handling config form submission: {:?}", form);
        for (name, config_value) in self.device_config.iter_mut() {
            info!("Processing config value: {}", name);
            let str_val = form.get(name).map(|s| s.as_str()).unwrap_or("").trim();
            if str_val.len() == 0 {
                if config_value.required {
                    log::error!("Missing required config value: {}", name);
                }
                else {
                    log::info!("Config value {} is None", name);
                    if ! config_value.value.is_none() {
                        log::info!("Setting optional config value {} to None", name);
                        config_value.value = config_value.value.to_none();
                        self.nvs.remove(name).ok();
                    }
                }
            }
            else {
                log::info!("Config value {} is {}", name, str_val);
                match config_value.value.from_str(str_val) {
                    Ok(new_value) => {
                        config_value.value = new_value;
                        // Save to NVS
                        match &config_value.value {
                            TypedValue::String(_len, Some(val)) => self.nvs.set_str(name, val).ok(),
                            TypedValue::Int32(Some(val)) => self.nvs.set_i32(name, *val).ok(),
                            TypedValue::Int64(Some(val)) => self.nvs.set_i64(name, *val).ok(),
                            _ => None,
                        };
                    }
                    Err(e) => {
                        log::error!("Failed to parse config value for {}: {}", name, e);
                    }
                }
            }
        }
        info!("Finished handling config form submission");
    }
}
