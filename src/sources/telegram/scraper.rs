use anyhow::anyhow;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;

use crate::events::Event;
use crate::sources::{fetch_url, create_client};

use super::TelegramScraperConfig;
use super::parser;

pub struct TelegramScraper {
    pub cfg: Arc<RwLock<TelegramScraperConfig>>,

    tx: mpsc::Sender<Event>,
    client: RwLock<reqwest::Client>,
    shutdown: CancellationToken,
}

impl TelegramScraper {
    pub async fn new(cfg: TelegramScraperConfig, tx: mpsc::Sender<Event>) -> anyhow::Result<Self> {
        tracing::info!("initializing listener {}", cfg.id);
        let client = create_client().await?;
        Ok(Self {
            cfg: Arc::new(RwLock::new(cfg)),
            tx,
            client: RwLock::new(client),
            shutdown: CancellationToken::new(),
        })
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        loop {
            let channel_url = self.cfg.read().await.channel_url.clone();

            tokio::select! {
                // Shutdown handler
                _ = self.shutdown.cancelled() => {
                    self.stop().await?;
                    return Ok(());
                }

                res = self.poll_cycle(&channel_url) => { res? }
            }
        }
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        let id = self.cfg.read().await.id.clone();
        tracing::info!("stopping listener with id {}", id);
        self.shutdown.cancel();
        Ok(())
    }

    /// Poll URL with sleep
    async fn poll_cycle(&self, url: &str) -> anyhow::Result<()> {
        let interval = self.cfg.read().await.poll_interval;
        match self.poll(url).await {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("poll failed, retrying: {e}");
                *self.client.write().await = create_client().await?;
                self.poll(url).await?;
            }
        }
        sleep(Duration::from_secs(interval.try_into().unwrap_or(600))).await;
        Ok(())
    }

    /// Poll URL, parses the channel info and posts,
    /// stores state in database, and sends webhook notifications.
    async fn poll(&self, url: &str) -> anyhow::Result<()> {
        let client = self.client.read().await;
        let html = fetch_url(&client, url).await?;
        let page = match parser::parse_page(&html)? {
            Some(p) => p,
            None => return Err(anyhow!("invalid channel: {}", url)),
        };

        let webhook_url = self.cfg.read().await.webhook_url.clone();
        self.tx.send(Event::NewPosts(page, webhook_url)).await?;

        Ok(())
    }
}
