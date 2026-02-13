use anyhow::Result;
use tokio::time::{Duration, sleep};
use std::fs;
use std::path::Path;

use config::Config;
use db::Db;

pub mod config;
mod db;
mod model;
mod web;

pub struct App {
    cfg: Config,
    db: Db,
    client: reqwest::Client,
}

impl App {
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

        Ok(Self { cfg, db, client })
    }

    pub async fn run(&self) -> Result<()> {
        tracing::info!("started listening to {}", &self.cfg.channel_url);
        loop {
            if let Err(e) = self.run_cycle().await {
                tracing::error!("post cycle failed: {e}");
            }

            sleep(Duration::from_secs(self.cfg.poll_interval)).await;
        }
    }

    async fn run_cycle(&self) -> Result<()> {
        let html = web::fetch_html(&self.client, &self.cfg.channel_url).await?;
        let page = web::parse_page(&html).await?;
        let mut new_posts = Vec::new();

        for post in &page.posts {
            if self.db.get_posts(&post.id)?.is_none() {
                tracing::info!("new post: {}", post.id);
                self.db.insert_post(post)?;
                new_posts.push(post.clone());
            }
        }
        
        if !new_posts.is_empty() {
            let res = web::send_webhook_retry(
                &self.client,
                &self.cfg.webhook_url,
                &page.channel,
                &new_posts,
                self.cfg.webhook_secret.as_deref(),
                5
            ).await;

            if let Err(e) = res {
                tracing::error!("webhook failed: {e}");
            }
        }

        Ok(())
    }
}
