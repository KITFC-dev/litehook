use serde::Deserialize;
use anyhow::Result;
use config::{Config as RawConfig, File};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub telegram: TelegramConfig,
    pub webhook: WebhookConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub poll_interval_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct TelegramConfig {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
}

pub fn load() -> Result<Config> {
    let cfg = RawConfig::builder()
        .add_source(File::with_name("config"))
        .build()?;

    Ok(cfg.try_deserialize()?)
}
