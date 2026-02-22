use anyhow::Result;
use serde::{Deserialize, Deserializer};

/// Litehook server configuration
#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_interval")]
    pub poll_interval: u64,

    #[serde(default = "default_db_path")]
    pub db_path: String,

    #[serde(deserialize_with = "deserialize_channels")]
    pub channels: Vec<String>,
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

fn default_interval() -> u64 {
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
            if s.starts_with("https://") {
                s.to_string()
            } else {
                format!("https://t.me/s/{}", s)
            }
        })
        .collect();

    Ok(channels)
}
