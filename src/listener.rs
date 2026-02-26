use anyhow::anyhow;
use rand::prelude::IndexedRandom;
use reqwest::Client;
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;

use crate::config::ListenerConfig;
use crate::db::Db;
use crate::model::{Channel, Post, WebhookPayload};
use crate::parser;

#[derive(Clone)]
pub struct Listener {
    pub cfg: ListenerConfig,

    db: Db,
    client: Client,
    shutdown: CancellationToken,
}

impl Listener {
    pub async fn new(
        cfg: ListenerConfig,
        db: Db,
        client_builder: reqwest::ClientBuilder,
    ) -> anyhow::Result<Self> {
        tracing::info!("initializing listener for {}", cfg.channel_url);
        let client = Self::configure_proxy(client_builder, &cfg.proxy_list_url).await?;
        Ok(Self {
            cfg,
            db,
            client,
            shutdown: CancellationToken::new(),
        })
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                _ = self.shutdown.cancelled() => {
                    self.stop().await?;
                    return Ok(());
                }

                res = self.poll_cycle(&self.cfg.channel_url) => { res? }
            }
        }
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        tracing::info!("stopping listening to {}", &self.cfg.channel_url);
        self.shutdown.cancel();
        Ok(())
    }

    /// Poll URL with wait
    async fn poll_cycle(&self, url: &str) -> anyhow::Result<()> {
        tracing::info!("polling {}", url);
        self.poll(url).await?;
        sleep(Duration::from_secs(self.cfg.poll_interval)).await;
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
            let res = self
                .send_webhook_retry(&self.cfg.webhook_url, &page.channel, &new_posts, 5)
                .await;

            if let Err(e) = res {
                tracing::error!("webhook failed: {e}");
            }
        }

        Ok(())
    }

    /// Create web client
    async fn configure_proxy(
        mut builder: reqwest::ClientBuilder,
        proxy_list_url: &Option<String>,
    ) -> anyhow::Result<reqwest::Client> {
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

        let res = self
            .client
            .post(url)
            .header(
                "x-secret",
                self.cfg.webhook_secret.clone().unwrap_or("".to_string()),
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
