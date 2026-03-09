use tokio::sync::{mpsc};
use crate::db::Db;
use crate::sources::Source;

use self::client::TelegramClient;
use self::scraper::TelegramScraper;

pub mod scraper;
pub mod client;

pub enum TelegramSourceKind {
    Scraper(TelegramScraper),
    Client(TelegramClient)
}

pub enum SourceConfig {
    TelegramScraper(TelegramScraperConfig),
    TelegramClient(TelegramClientConfig),
}

#[derive(Debug, Clone)]
pub struct TelegramScraperConfig {
    pub id: String,
    pub channel_url: String,
    pub poll_interval: i64,
    pub webhook_url: String,
    pub proxy_list_url: Option<String>,
}

pub struct TelegramClientConfig {
    pub id: String,
    pub api_id: i32,
    pub api_hash: String,
    pub session_file: String,
    pub channels: Vec<TelegramChannelConfig>,
}

pub struct TelegramChannelConfig {
    pub id: i64,
    pub webhook_url: String,
}

pub struct TelegramSource {
    kind: TelegramSourceKind
}

impl TelegramSource {
    pub async fn new(cfg: SourceConfig, db: Db) -> anyhow::Result<Self> {
        let kind = match cfg {
            // Scraper
            SourceConfig::TelegramScraper(cfg) => {
                TelegramSourceKind::Scraper(TelegramScraper::new(cfg, db).await?)
            }

            // Client
            SourceConfig::TelegramClient(cfg) => {
                TelegramSourceKind::Client(TelegramClient::new(cfg).await?)
            }
        };

        Ok(Self { kind })
    }
}

#[async_trait::async_trait]
impl Source for TelegramSource {
    fn name(&self) -> &'static str { 
        "telegram"
    }

    #[allow(unused)]
    async fn run(&self, tx: mpsc::Sender<String>) -> anyhow::Result<()> {
        todo!()
    }

    async fn stop(&self) {
        todo!()
    }
}
