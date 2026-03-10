use crate::sources::{Source, SourceConfig};
use serde::Deserialize;
use tokio::sync::mpsc;

use self::client::TelegramClient;
use self::scraper::TelegramScraper;

pub mod client;
pub mod scraper;

pub enum TelegramSourceKind {
    Scraper(TelegramScraper),
    Client(TelegramClient),
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramScraperConfig {
    pub id: String,
    pub channel_url: String,
    pub poll_interval: i64,
    pub webhook_url: String,
    pub proxy_list_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramClientConfig {
    pub id: String,
    pub api_id: i32,
    pub api_hash: String,
    pub session_file: String,
    pub channels: Vec<TelegramChannelConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramChannelConfig {
    pub id: i64,
    pub webhook_url: String,
}

pub struct TelegramSource {
    kind: TelegramSourceKind,

    #[allow(unused)]
    tx: mpsc::Sender<String>,
}

impl TelegramSource {
    pub async fn new(cfg: SourceConfig, tx: mpsc::Sender<String>) -> anyhow::Result<Self> {
        let kind = match cfg.kind.as_str() {
            "telegram_scraper" => {
                let cfg: TelegramScraperConfig = serde_json::from_value(cfg.raw)?;
                TelegramSourceKind::Scraper(TelegramScraper::new(cfg, tx.clone()).await?)
            }
            "telegram_client" => {
                let cfg: TelegramClientConfig = serde_json::from_value(cfg.raw)?;
                TelegramSourceKind::Client(TelegramClient::new(cfg, tx.clone()).await?)
            }
            other => anyhow::bail!("unknown kind: {other}"),
        };

        Ok(Self { kind, tx })
    }
}

#[async_trait::async_trait(?Send)]
impl Source for TelegramSource {
    fn name(&self) -> &'static str {
        "telegram"
    }

    #[allow(unused)]
    async fn run(&self) -> anyhow::Result<()> {
        match &self.kind {
            TelegramSourceKind::Scraper(scraper) => scraper.run().await,
            TelegramSourceKind::Client(client) => client.run().await,
        }
    }

    async fn stop(&self) {}
}
