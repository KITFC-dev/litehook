use serde::Deserialize;
use anyhow::Result;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Config {
    #[serde(default = "default_interval")]
    pub poll_interval: u64,
    pub channel_url: String,
    pub webhook_url: String,
    pub webhook_secret: Option<String>,
}

impl Config {
    pub fn from_dotenv() -> Result<Self> {
        dotenvy::dotenv().ok();
        Ok(envy::from_env()?)
    }
}

fn default_interval() -> u64 { 600 }
