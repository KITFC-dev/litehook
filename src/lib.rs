use anyhow::Result;
use tokio::time::{Duration, sleep};
use futures::stream::{FuturesUnordered, StreamExt};
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
        let mut tasks = FuturesUnordered::new();

        for post in &page.posts {
            let p = self.db.get_posts(&post.id)?;

            if p.is_none() {
                tracing::info!("new post: {}", post.id);
                self.db.insert_post(post)?;
                
                let client = self.client.clone();
                let url = self.cfg.webhook_url.clone();
                let secret = self.cfg.webhook_secret.clone();

                tasks.push( async move { 
                    web::send_webhook_retry(
                        &client,
                        &url,
                        post, 
                        secret.as_deref(),
                        5
                    ).await
                });
            }
        }
        
        while let Some(result) = tasks.next().await {
            if let Err(e) = result {
                tracing::error!("webhook failed: {:?}", e);
            }
        }

        Ok(())
    }
}
