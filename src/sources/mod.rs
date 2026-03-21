use rand::prelude::IndexedRandom;
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::FromRow;

use crate::config;

pub mod registry;
pub mod telegram;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SourceConfig {
    pub id: String,
    pub kind: String,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub id: String,
    pub kind: String,
    pub raw: serde_json::Value,
    pub active: bool,
}

impl From<SourceConfig> for SourceInfo {
    fn from(cfg: SourceConfig) -> Self {
        Self {
            id: cfg.id,
            kind: cfg.kind,
            raw: cfg.raw,
            active: false,
        }
    }
}

/// Source trait
#[async_trait::async_trait]
pub trait Source: Send + Sync {
    /// Get the id of the source
    fn id(&self) -> &str;

    /// Source Name
    fn name(&self) -> &'static str;

    /// Run the source
    async fn run(&self) -> anyhow::Result<()>;

    /// Stop the source
    async fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Fetch SOCKS5 proxy list, and create proxy config
async fn get_proxy(proxy_list_url: &str) -> anyhow::Result<String> {
    let res = reqwest::Client::new()
        .get(proxy_list_url)
        .send()
        .await?
        .text()
        .await?;
    let mut rng = rand::rng();
    let proxy_addr: Vec<&str> = res
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let proxy_addr = proxy_addr
        .choose(&mut rng)
        .ok_or_else(|| anyhow::anyhow!("failed to fetch proxy"))?;
    Ok(proxy_addr.to_string())
}

/// Create web client
async fn create_client() -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .timeout(tokio::time::Duration::from_secs(30))
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ));

    // Configure proxy
    if let Some(url) = &config::get_env().proxy_list_url {
        let addr = get_proxy(url).await?;
        tracing::info!("using proxy address {}", addr);
        builder = builder.proxy(reqwest::Proxy::all(format!("socks5h://{}", addr))?);
    };

    Ok(builder.build()?)
}

/// Helper for fetching URL
pub async fn fetch_url(client: &reqwest::Client, url: &str) -> anyhow::Result<String> {
    Ok(client.get(url).send().await?.text().await?)
}

/// Helper for deserializing channels separated by commas.
///
/// Outputs urls of channels, like this: https://t.me/s/channel
pub fn deserialize_channels_url<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let items = deserialize_items(deserializer)?;

    let channels = items
        .into_iter()
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

/// Helper for deserializing items separated by commas.
pub fn deserialize_items<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        String::deserialize(deserializer)?
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    )
}
