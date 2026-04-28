use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use crate::events::Event;
use crate::sources::registry::SourceRegistration;
use crate::sources::{Source, SourceConfig, deserialize_items};

use self::client::TelegramClient;
use self::scraper::TelegramScraper;

pub mod client;
pub mod parser;
pub mod scraper;

pub const KIND_SCRAPER: &str = "telegram_scraper";
pub const KIND_CLIENT: &str = "telegram_client";

pub enum TelegramSourceKind {
    Scraper(TelegramScraper),
    Client(Mutex<TelegramClient>),
}

/// Config for Telegram scraper
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct TelegramScraperConfig {
    pub id: String,
    pub channel_url: String,
    pub poll_interval: i64,
    pub webhook_url: String,
}

/// Config for Telegram client
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TelegramClientConfig {
    pub id: String,
    pub api_id: i32,
    pub api_hash: String,
    pub phone_number: String,
    pub webhook_url: String,

    #[serde(deserialize_with = "deserialize_items")]
    pub channel_ids: Vec<String>,
}

/// Representation of Telegram channel
/// For specifying channels in client
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TelegramChannelConfig {
    pub id: i64,
    pub webhook_url: String,
}

/// Telegram source
///
/// Can be either a scraper or a client
pub struct TelegramSource {
    id: String,
    kind: TelegramSourceKind,
}

impl TelegramSource {
    pub async fn new(cfg: SourceConfig, tx: mpsc::Sender<Event>) -> anyhow::Result<Self> {
        let kind = match cfg.kind.as_str() {
            KIND_SCRAPER => {
                let scraper_cfg: TelegramScraperConfig = serde_json::from_value(cfg.raw.clone())?;
                TelegramSourceKind::Scraper(TelegramScraper::new(scraper_cfg, tx).await?)
            }
            KIND_CLIENT => {
                let client_cfg: TelegramClientConfig = serde_json::from_value(cfg.raw.clone())?;
                TelegramSourceKind::Client(Mutex::new(TelegramClient::new(client_cfg, tx)))
            }
            other => anyhow::bail!("unknown telegram kind: {other}"),
        };

        Ok(Self {
            id: cfg.id.clone(),
            kind,
        })
    }
}

#[async_trait::async_trait]
impl Source for TelegramSource {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &'static str {
        "telegram"
    }

    async fn run(&self) -> anyhow::Result<()> {
        match &self.kind {
            TelegramSourceKind::Scraper(scraper) => scraper.run().await,
            TelegramSourceKind::Client(client) => client.lock().await.run().await,
        }
    }

    async fn stop(&self) -> anyhow::Result<()> {
        match &self.kind {
            TelegramSourceKind::Scraper(scraper) => scraper.stop().await,
            TelegramSourceKind::Client(client) => client.lock().await.stop().await,
        }
    }
}

// Register sources
inventory::submit!(SourceRegistration {
    kind: KIND_SCRAPER,
    name: "Telegram scraper",
    fields: || schemars::schema_for!(TelegramScraperConfig),
    factory: |cfg, tx| Box::pin(async move {
        Ok(Box::new(TelegramSource::new(cfg, tx).await?) as Box<dyn Source + Send>)
    }),
});

inventory::submit!(SourceRegistration {
    kind: KIND_CLIENT,
    name: "Telegram client",
    fields: || schemars::schema_for!(TelegramClientConfig),
    factory: |cfg, tx| Box::pin(async move {
        Ok(Box::new(TelegramSource::new(cfg, tx).await?) as Box<dyn Source + Send>)
    }),
});
