use crate::{Feature, config::{Config, ConfigManager, ConfigValue, FeatureConfig, FeatureDescriptor, TypedValue}};

pub const USER_NAME: &str = "user_name";
pub const PASSWORD: &str = "password";
pub const HOSTNAME: &str = "hostname";
pub const BASE_SERVICE_URL: &str = "base_url";
pub const GET_IP_URL: &str = "get_ip_url";
pub const GET_REQUIRES_STRIP: &str = "get_req_strip";
pub const UPDATE_URL: &str = "update_url";
pub const UPDATE_REQUIRES_ADDRESS: &str = "upd_req_addr";
pub const UPDATE_INTERVAL: &str = "upd_int";

// pub struct DynDns2Config {
//     user_name: String,
//     password: String,
//     hostname: String,
//     base_service_url: String,
//     get_ip_url: Option<String>,
//     get_requires_strip: bool,
//     update_url: Option<String>,
//     update_requires_address: bool,
//     update_interval: u64,
// }

// impl DynDns2Config {
//     pub fn new(config_manager: &ConfigManager) -> anyhow::Result<Self> {
//         Ok(Self {
//             user_name: config_manager.get(USER_NAME)?.unwrap_or_default(),
//             password: config_manager.get(PASSWORD)?.unwrap_or_default(),
//             hostname: config_manager.get(HOSTNAME)?.unwrap_or_default(),
//             base_service_url: config_manager.get(BASE_SERVICE_URL)?.unwrap_or_default(),
//             get_ip_url: config_manager.get(GET_IP_URL)?,
//             get_requires_strip: config_manager.get(GET_REQUIRES_STRIP)?.unwrap_or(false),
//             update_url: config_manager.get(UPDATE_URL)?,
//             update_requires_address: config_manager.get(UPDATE_REQUIRES_ADDRESS)?.unwrap_or(false),
//             update_interval: config_manager.get(UPDATE_INTERVAL)?.unwrap_or(3600),
//         })
//     }
// }

pub struct DynDns2 {
}

impl DynDns2 {
    pub fn new() -> Self {
        Self {
        }
    }
}

impl Feature for DynDns2 {
    fn create_descriptor(&self) -> anyhow::Result<FeatureDescriptor> {
        let mut config = Config::new();
        config.insert(USER_NAME.to_string(), ConfigValue { value: TypedValue::String(32, None), required: true })?;
        config.insert(PASSWORD.to_string(), ConfigValue { value: TypedValue::String(32, None), required: true })?;
        config.insert(HOSTNAME.to_string(), ConfigValue { value: TypedValue::String(32, None), required: true })?;
        config.insert(BASE_SERVICE_URL.to_string(), ConfigValue { value: TypedValue::String(32, None), required: true })?;
        config.insert(GET_IP_URL.to_string(), ConfigValue { value: TypedValue::String(32, None), required: true })?;
        config.insert(GET_REQUIRES_STRIP.to_string(), ConfigValue { value: TypedValue::Bool(false), required: true })?;
        config.insert(UPDATE_URL.to_string(), ConfigValue { value: TypedValue::String(32, None), required: true })?;
        config.insert(UPDATE_REQUIRES_ADDRESS.to_string(), ConfigValue { value: TypedValue::Bool(false), required: false })?;
        config.insert(UPDATE_INTERVAL.to_string(), ConfigValue { value: TypedValue::Int64(Some(3600)), required: true })?;
        Ok(FeatureDescriptor {
            name: "DynDNS2".to_string(),
            config,
        })
    }
}