//! litehook
//!
//! Polls a public Telegram channel page and sends webhook notifications
//! when new posts are detected. State is stored in SQLite database.

use anyhow::{Ok, Result, anyhow};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::select;
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;

use crate::model::{Channel, Post, WebhookPayload};
use config::Config;
use db::Db;

pub mod config;
mod db;
mod model;
mod parser;

/// Core application state for the Litehook server.
///
/// Holds configuration, database connection, HTTP client,
/// and shutdown signal.
pub struct App {
    /// Tokio Cancellation token for shutdown signal
    pub shutdown: CancellationToken,

    cfg: Config,
    db: Db,
    client: reqwest::Client,
}

impl App {
    /// Create a new instance of [App].
    ///
    /// Creates SQLite database in data/litehook.db and creates data dir
    /// if it doesn't exist. HTTP client is configured with a 10 second timeout.
    pub async fn new(cfg: Config) -> Result<Self> {
        tracing::info!("initializing");
        fs::create_dir_all(Path::new("data"))?;
        if !Path::new("data/litehook.db").exists() {
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open("data/litehook.db")?;
        }
        let db = Db::new("data/litehook.db").await?;
        let client = Self::create_client(&cfg.proxy_list_url).await?;

        Ok(Self {
            shutdown: CancellationToken::new(),
            cfg,
            db,
            client,
        })
    }

    async fn create_client(proxy_url: &Option<String>) -> Result<reqwest::Client> {
        // Fetch SOCKS5 proxy list, and create proxy config
        let proxy = if let Some(url) = proxy_url {
            tracing::info!("configuring proxy");
            let res = reqwest::Client::new().get(url).send().await?.text().await?;
            let proxy_addr = res
                .lines()
                .next()
                .map(|s| s.trim())
                .ok_or(anyhow!("failed to fetch proxy"))?;
            Some(reqwest::Proxy::all(format!("socks5h://{}", proxy_addr))?)
        } else {
            None
        };

        // Create client
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(format!(
                "{}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ));

        if let Some(proxy) = proxy {
            builder = builder.proxy(proxy);
        }

        let client = builder.build()?;

        Ok(client)
    }

    /// Start listening to channels.
    pub async fn run(self: Arc<Self>) -> Result<()> {
        tracing::info!(
            "started listening to {} channels",
            &self.cfg.channels.len()
        );
        let local = tokio::task::LocalSet::new();

        local
            .run_until(async move {
                let mut handles = Vec::new();

                for url in &self.cfg.channels {
                    let app = Arc::clone(&self);
                    let url = url.clone();
                    let handle =
                        tokio::task::spawn_local(async move { app.listen_channel(&url).await });
                    handles.push(handle);
                }

                for h in handles {
                    let _ = h.await;
                }
            })
            .await;

        Ok(())
    }

    /// Poll loop, handles shutdown signal
    async fn listen_channel(&self, url: &str) -> Result<()> {
        loop {
            select! {
                _ = self.shutdown.cancelled() => {
                    tracing::info!("stopped listening to {}", url);
                    return Ok(());
                }

                res = self.poll_cycle(url) => {
                    if let Err(e) = res {
                        tracing::error!("poll failed: {e}");
                        return Err(e);
                    }
                }
            }
        }
    }

    /// Poll URL with wait
    async fn poll_cycle(&self, url: &str) -> Result<()> {
        self.poll(url).await?;
        sleep(Duration::from_secs(self.cfg.poll_interval)).await;
        Ok(())
    }

    /// Poll URL, parses the channel info and posts,
    /// stores state in database, and sends webhook notifications.
    async fn poll(&self, url: &str) -> Result<()> {
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

    async fn send_webhook(
        &self,
        url: &str,
        channel: &Channel,
        new_posts: &[Post],
    ) -> Result<reqwest::Response> {
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
    ) -> Result<reqwest::Response> {
        for att in 1..=max_retries {
            match self.send_webhook(url, channel, new_posts).await {
                std::result::Result::Ok(res) => return Ok(res),
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
