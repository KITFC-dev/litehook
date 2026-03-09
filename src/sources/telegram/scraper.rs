use anyhow::anyhow;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;

use crate::parser;

use super::TelegramScraperConfig;

pub struct TelegramScraper {
    pub cfg: Arc<RwLock<TelegramScraperConfig>>,

    tx: mpsc::Sender<String>,
    client: RwLock<reqwest::Client>,
    shutdown: CancellationToken,
}

impl TelegramScraper {
    pub async fn new(cfg: TelegramScraperConfig, tx: mpsc::Sender<String>) -> anyhow::Result<Self> {
        //TODO: cfg.validate()?;
        tracing::info!("initializing listener {}", cfg.id);
        let client = Self::create_client().await?;
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
        tracing::info!("stopping listener {}", id);
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
                #[allow(unused)]
                let proxy_list_url = self.cfg.read().await.proxy_list_url.clone();
                *self.client.write().await = Self::create_client().await?;
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
        let html = parser::fetch_html(&client, url).await?;
        let page = match parser::parse_page(&html).await? {
            Some(p) => p,
            None => return Err(anyhow!("invalid channel: {}", url)),
        };
        // TODO: use event handler for this
        // TODO: send event to event handler
        // let mut new_posts = Vec::new();

        // for post in &page.posts {
        //     if self.db.get_posts(&post.id).await?.is_none() {
        //         tracing::info!("new post: {}", post.id);
        //         self.db.insert_post(post).await?;
        //         new_posts.push(post.clone());
        //     }
        // }

        // if !new_posts.is_empty() {
        //     tracing::info!("new posts: {}", new_posts.len());
        // }

        Ok(())
    }

    /// Create web client
    async fn create_client() -> anyhow::Result<reqwest::Client> {
        let builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(format!(
                "{}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ));

        let client = builder.build()?;

        Ok(client)
    }
}
