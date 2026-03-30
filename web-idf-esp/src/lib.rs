use crate::config::{Config, FeatureConfig, FeatureDescriptor};

pub const SSID_LEN: usize = 32;
pub const PASSWORD_LEN: usize = 64;
pub const HOSTNAME_LEN: usize = 32;
pub const FQDN_LEN: usize = 64;

pub mod sparko_cyd;

mod config;
mod wifi;
mod http;
mod portal;
mod led;
mod tz;
pub mod dyndns2;

// trait Task {
//     fn run(&self, sparko_cyd: &sparko_cyd::SparkoCyd) -> anyhow::Result<u64>;
// }

pub trait Feature {
    // fn start(&self, sparko_cyd: &sparko_cyd::SparkoCyd) -> anyhow::Result<()>;
    fn create_descriptor(&self) -> anyhow::Result<FeatureDescriptor>;
}