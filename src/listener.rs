use anyhow::anyhow;
use rand::prelude::IndexedRandom;
use std::sync::Arc;
use tokio::sync::{RwLock, watch};
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;

use crate::config::{GlobalListenerConfig, ListenerConfig};
use crate::db::Db;
use crate::model::{Channel, Post, WebhookPayload};
use crate::parser;

pub struct Listener {
    pub cfg: Arc<RwLock<ListenerConfig>>,

    db: Db,
    client: reqwest::Client,
    shutdown: CancellationToken,
}

impl Listener {
    pub async fn new(cfg: ListenerConfig, db: Db) -> anyhow::Result<Self> {
        cfg.validate()?;
        tracing::info!("initializing listener {}", cfg.id);
        let client = Self::create_client(&cfg.proxy_list_url).await?;
        Ok(Self {
            cfg: Arc::new(RwLock::new(cfg)),
            db,
            client,
            shutdown: CancellationToken::new(),
        })
    }

    pub async fn run(
        &self,
        mut global_cfg: watch::Receiver<GlobalListenerConfig>,
    ) -> anyhow::Result<()> {
        loop {
            let channel_url = self.cfg.read().await.channel_url.clone();

            tokio::select! {
                // Shutdown handler
                _ = self.shutdown.cancelled() => {
                    self.stop().await?;
                    return Ok(());
                }

                // Global config change handler
                _ = global_cfg.changed() => {
                    let current_cfg = self.cfg.read().await.clone();
                    self.reconfigure(&global_cfg.borrow_and_update().clone(), current_cfg).await;
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

    pub async fn reconfigure(
        &self,
        global_cfg: &GlobalListenerConfig,
        listener_cfg: ListenerConfig,
    ) {
        tracing::info!("reconfiguring listener {}", listener_cfg.id);
        *self.cfg.write().await = listener_cfg.merge_with(global_cfg);
    }

    /// Poll URL with wait
    async fn poll_cycle(&self, url: &str) -> anyhow::Result<()> {
        let interval = self.cfg.read().await.poll_interval.unwrap_or(600);
        self.poll(url).await?;
        sleep(Duration::from_secs(interval.try_into().unwrap())).await;
        Ok(())
    }

    /// Poll URL, parses the channel info and posts,
    /// stores state in database, and sends webhook notifications.
    async fn poll(&self, url: &str) -> anyhow::Result<()> {
        let html = parser::fetch_html(&self.client, url).await?;
        let page = match parser::parse_page(&html).await? {
            Some(p) => p,
            None => return Err(anyhow!("invalid channel: {}", url)),
        };
        let mut new_posts = Vec::new();

        for post in &page.posts {
            if self.db.get_posts(&post.id).await?.is_none() {
                tracing::info!("new post: {}", post.id);
                self.db.insert_post(post).await?;
                new_posts.push(post.clone());
            }
        }

        if !new_posts.is_empty() {
            let webhook_url = self
                .cfg
                .read()
                .await
                .webhook_url
                .clone()
                .ok_or(anyhow!("webhook_url is not configured"))?;
            let res = self
                .send_webhook_retry(&webhook_url, &page.channel, &new_posts, 5)
                .await;

            if let Err(e) = res {
                tracing::error!("webhook failed: {e}");
            }
        }

        Ok(())
    }

    /// Create web client
    async fn create_client(proxy_list_url: &Option<String>) -> anyhow::Result<reqwest::Client> {
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(format!(
                "{}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ));

        if let Some(url) = proxy_list_url {
            tracing::info!("configuring proxy");
            let addr = get_proxy(url).await?;
            builder = builder.proxy(reqwest::Proxy::all(format!("socks5h://{}", addr))?);
        };

        let client = builder.build()?;

        Ok(client)
    }

    async fn send_webhook(
        &self,
        url: &str,
        channel: &Channel,
        new_posts: &[Post],
    ) -> anyhow::Result<reqwest::Response> {
        let payload = WebhookPayload { channel, new_posts };
        let webhook_secret = self.cfg.read().await.webhook_secret.clone();

        let res = self
            .client
            .post(url)
            .header(
                "x-secret",
                &webhook_secret.clone().unwrap_or("".to_string()),
            )
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(anyhow!(res.status()));
        }

        Ok(res)
    }

    async fn send_webhook_retry(
        &self,
        url: &str,
        channel: &Channel,
        new_posts: &[Post],
        max_retries: u64,
    ) -> anyhow::Result<reqwest::Response> {
        for att in 1..=max_retries {
            match self.send_webhook(url, channel, new_posts).await {
                Ok(res) => return Ok(res),
                Err(e) if att < max_retries => {
                    tracing::warn!("webhook failed ({}/{}): {}", att, max_retries, e);
                    sleep(Duration::from_secs(1)).await;
                }
                Err(e) => {
                    tracing::error!("webhook failed after {} attempts: {}", max_retries, e);
                    return Err(e);
                }
            }
        }

        Err(anyhow!("webhook failed"))
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
        .ok_or_else(|| anyhow!("failed to fetch proxy"))?;
    Ok(proxy_addr.to_string())
}
