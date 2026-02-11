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

fn validate(cfg: &Config) -> Result<()> {
    if cfg.server.poll_interval_seconds <= 0 {
        return Err(anyhow::anyhow!("poll_interval_seconds must be greater than 0"));
    }
    if cfg.telegram.url.is_empty() || !cfg.telegram.url.starts_with("https://") {
        return Err(anyhow::anyhow!("telegram url cannot be empty and must start with https://"));
    }
    if cfg.webhook.url.is_empty() || !cfg.webhook.url.starts_with("https://") {
        return Err(anyhow::anyhow!("webhook url cannot be empty and must start with https://"));
    }
    Ok(())
}

pub fn load() -> Result<Config> {
    let rawcfg = RawConfig::builder()
        .add_source(File::with_name("config"))
        .build()?;

    let cfg: Config = rawcfg.try_deserialize()?;
    validate(&cfg)?;
    Ok(cfg)
}
