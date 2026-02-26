//! litehook
//!
//! Polls a public Telegram channel page and sends webhook notifications
//! when new posts are detected. State is stored in SQLite database.

use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

use config::{Config, ListenerConfig};
use db::Db;
use listener::Listener;

pub mod config;
mod db;
mod listener;
mod model;
mod parser;

/// Core application state for the Litehook server.
///
/// Holds configuration, database connection, HTTP client,
/// and shutdown signal.
pub struct App {
    /// Tokio Cancellation token for shutdown signal
    pub shutdown: CancellationToken,

    listeners: Mutex<HashMap<String, Arc<Listener>>>,
    cfg: Config,
    db: Db,
}

impl App {
    /// Create a new instance of [App].
    ///
    /// Creates SQLite database in data/litehook.db and creates data dir
    /// if it doesn't exist. HTTP client is configured with a 10 second timeout.
    pub async fn new(cfg: Config) -> anyhow::Result<Self> {
        tracing::info!("initializing");
        let db = Db::new(&cfg.db_path).await?;

        Ok(Self {
            shutdown: CancellationToken::new(),
            listeners: Mutex::new(HashMap::new()),
            cfg,
            db,
        })
    }

    /// Run [App], spawns listener local tasks and handles shutdown signal.
    pub async fn run(self: Arc<Self>) -> anyhow::Result<()> {
        tracing::info!("adding {} listeners", &self.cfg.channels.len());
        // Local set is needed because scraper is !Send
        let local = tokio::task::LocalSet::new();

        local
            .run_until(async {
                for url in &self.cfg.channels {
                    let test_cfg = ListenerConfig {
                        id: url.clone(),
                        poll_interval: self.cfg.poll_interval,
                        channel_url: url.clone(),
                        proxy_list_url: self.cfg.proxy_list_url.clone(),
                        webhook_url: self.cfg.webhook_url.clone(),
                        webhook_secret: self.cfg.webhook_secret.clone(),
                    };
                    self.add_listener(test_cfg).await;
                }

                tokio::select! {
                    _ = self.shutdown.cancelled() => {
                        self.stop().await
                    }
                }
            })
            .await;

        Ok(())
    }

    /// Stop all [Listener]s and shutdown the server.
    pub async fn stop(&self) {
        tracing::info!("stopping all listeners");
        let mut listeners = self.listeners.lock().await;
        for (_, listener) in listeners.drain() {
            if let Err(e) = listener.stop().await {
                tracing::error!("failed to stop listener: {e}");
            }
        }
    }

    /// Add a new [Listener] to the server.
    pub async fn add_listener(&self, cfg: ListenerConfig) {
        tracing::info!("adding listener for channel {}", cfg.channel_url);
        let client_builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(format!(
                "{}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ));

        let id = cfg.id.clone();
        let listener = match Listener::new(cfg, self.db.clone(), client_builder).await {
            Ok(listener) => Arc::new(listener),
            Err(e) => {
                tracing::error!("failed to create listener: {e}");
                return;
            }
        };

        self.listeners
            .lock()
            .await
            .insert(id, Arc::clone(&listener));

        tokio::task::spawn_local(async move { listener.run().await });
    }

    /// Remove a [Listener] from the server.
    pub async fn remove_listener(&self, url: &str) {
        let mut listeners = self.listeners.lock().await;
        if let Some(listener) = listeners.remove(url) {
            if let Err(e) = listener.stop().await {
                tracing::error!("failed to stop listener: {e}");
            }
        } else {
            tracing::warn!("listener not found for channel {}", url);
        }
    }

    /// Update a [Listener]
    /// 
    /// Works by removing the old listener and adding a new one
    /// with the updated configuration. Maybe can be improved in the future.
    #[allow(unused)]
    pub async fn update_listener(&self, cfg: ListenerConfig) {
        self.remove_listener(&cfg.id).await;
        self.add_listener(cfg).await;
    }
}
