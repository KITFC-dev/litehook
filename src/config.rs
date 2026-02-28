use anyhow::Result;
use serde::{Deserialize, Deserializer};

/// Litehook server configuration
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_interval")]
    pub poll_interval: i64,

    #[serde(default = "default_db_path")]
    pub db_path: String,

    #[serde(deserialize_with = "deserialize_channels")]
    pub channels: Vec<String>,
    pub proxy_list_url: Option<String>,
    pub webhook_url: String,
    pub webhook_secret: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ListenerConfig {
    pub id: String,

    #[serde(default = "default_interval")]
    pub poll_interval: i64,
    pub channel_url: String,
    pub proxy_list_url: Option<String>,
    pub webhook_url: String,
    pub webhook_secret: Option<String>,
}

impl Config {
    /// Create a new instance of [Config] with environment variables
    pub fn from_dotenv() -> Result<Self> {
        dotenvy::dotenv().ok();
        Ok(envy::from_env()?)
    }
}

fn default_port() -> u16 {
    4101
}

fn default_interval() -> i64 {
    600
}

fn default_db_path() -> String {
    "data/litehook.db".to_string()
}

fn deserialize_channels<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;

    let channels = raw
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            if s.starts_with("https://t.me/s/") {
                s[8..].to_string()
            } else {
                s.to_string()
            }
        })
        .collect();

    Ok(channels)
}
