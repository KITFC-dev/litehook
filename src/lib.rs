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
        let db = Db::new("data/posts.db")?;
        let client = reqwest::Client::new();

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
        let posts = web::parse_posts(&html).await?;

        for post in &posts {
            let p = self.db.get_posts(&post.id)?;
            if p.is_none() {
                tracing::info!("new post: {}", post.id);
                self.db.insert_post(post)?;
                web::send_webhook(&self.client, &self.cfg.webhook_url, post, self.cfg.webhook_secret.as_deref()).await?;
            }
        }

        Ok(())
    }
}
