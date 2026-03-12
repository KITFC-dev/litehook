use reqwest::Client;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;

use crate::db::Db;
use crate::model::{Channel, Page, Post, WebhookPayload};

/// Event type
#[derive(Debug)]
pub enum Event {
    NewPost(Post),
    NewPosts(Page, String),
}

pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
    db: Db,
    client: Client,
    webhook_secret: Option<String>,
    shutdown: CancellationToken,
}

impl EventHandler {
    pub fn new(rx: mpsc::Receiver<Event>, db: Db) -> Self {
        Self {
            rx,
            db,
            client: Client::new(),
            webhook_secret: None,
            shutdown: CancellationToken::new(),
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                _ = self.shutdown.cancelled() => {
                    self.rx.close();
                    return;
                }
                Some(event) = self.rx.recv() => {
                    if let Err(e) = self.handle_event(event).await {
                        tracing::error!("error while handling event: {e}");
                    }
                }
            }
        }
    }

    pub async fn handle_event(&mut self, event: Event) -> anyhow::Result<()> {
        #[allow(clippy::single_match)]
        match event {
            Event::NewPosts(page, cfg) => self.handle_new_posts(&page, &cfg).await?,

            _ => (),
        }

        Ok(())
    }

    pub async fn handle_new_posts(&self, page: &Page, webhook_url: &str) -> anyhow::Result<()> {
        let mut new_posts = Vec::new();

        // Filter for new posts
        for post in &page.posts {
            if self.db.get_posts(&post.id).await?.is_none() {
                tracing::info!("new post: {}", post.id);
                self.db.insert_post(post).await?;
                new_posts.push(post.clone());
            }
        }

        // Send webhook
        if !new_posts.is_empty() {
            self.send_webhook_retry(webhook_url, &page.channel, &new_posts, 5)
                .await?;
        }

        Ok(())
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
                &self.webhook_secret.clone().unwrap_or("".to_string()),
            )
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(res.status()));
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

        Err(anyhow::anyhow!("webhook failed"))
    }

    pub async fn stop(mut self) {
        self.shutdown.cancel();
        self.rx.close();
    }
}
