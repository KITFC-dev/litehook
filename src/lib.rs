//! litehook
//!
//! Polls a public Telegram channel page and sends webhook notifications
//! when new posts are detected. State is stored in SQLite database.

use anyhow::Result;
use tokio::select;
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;
use anyhow::anyhow;
use std::fs;
use std::path::Path;

use config::Config;
use db::Db;
use crate::model::{Channel, Post, WebhookPayload};

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
        let db = Db::new("data/litehook.db")?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(format!(
                "{}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;

        Ok(Self { cfg, db, client, shutdown: CancellationToken::new() })
    }

    /// Starts the main polling loop.
    pub async fn run(&self) -> Result<()> {
        tracing::info!("started listening to {}", &self.cfg.channel_url);
        loop {
            select! {
                _ = self.shutdown.cancelled() => {
                    tracing::info!("exiting loop");
                    return Ok(());
                }

                _ = async {
                    if let Err(e) = self.poll_channel().await {
                        tracing::error!("post cycle failed: {e}");
                    }

                    sleep(Duration::from_secs(self.cfg.poll_interval)).await;
                } => {}
            }
        }
    }

    /// Poll channel for new posts,
    /// parses the channel and posts, 
    /// stores state in database, and sends webhook notifications.
    async fn poll_channel(&self) -> Result<()> {
        let html = parser::fetch_html(&self.client, &self.cfg.channel_url).await?;
        let page = parser::parse_page(&html).await?;
        let mut new_posts = Vec::new();

        for post in &page.posts {
            if self.db.get_posts(&post.id)?.is_none() {
                tracing::info!("new post: {}", post.id);
                self.db.insert_post(post)?;
                new_posts.push(post.clone());
            }
        }
        
        if !new_posts.is_empty() {
            let res = self.send_webhook_retry(
                &self.cfg.webhook_url,
                &page.channel,
                &new_posts,
                5
            ).await;

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
        new_posts: &Vec<Post>
    ) -> Result<reqwest::Response> 
    {
        let payload = WebhookPayload {
            channel,
            new_posts
        };
        
        let res = self.client
            .post(url)
            .header("x-secret", self.cfg.webhook_secret.clone().unwrap_or("".to_string()))
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
        new_posts: &Vec<Post>,
        max_retries: u64
    ) -> Result<reqwest::Response> 
    {
        for att in 1..=max_retries {
            let res = self.send_webhook(url, channel, new_posts).await;
            if res.is_ok() {
                return res;
            } else if att < max_retries {
                tracing::warn!("webhook failed ({}/{}): {}", att, max_retries, res.unwrap_err());
                sleep(Duration::from_secs(1)).await;
            }
        }

        Err(anyhow!("webhook failed after {} attempts", max_retries))
    }
}
