//! litehook
//!
//! Polls a public Telegram channel page and sends webhook notifications
//! when new posts are detected. State is stored in SQLite database.

use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

use config::{Config, ListenerConfig};
use db::Db;

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

    listeners: Mutex<HashMap<String, Arc<listener::Listener>>>,
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

    /// Run [App], spawns listeners and starts listening to configuration changes.
    pub async fn run(self: Arc<Self>) -> anyhow::Result<()> {
        tracing::info!("started listening to {} channels", &self.cfg.channels.len());
        let local = tokio::task::LocalSet::new();

        local
            .run_until(async move {
                let mut set = JoinSet::new();

                for url in &self.cfg.channels {
                    let app = Arc::clone(&self);
                    let url = url.clone();
                    let client_builder = reqwest::Client::builder()
                        .timeout(Duration::from_secs(30))
                        .user_agent(format!(
                            "{}/{}",
                            env!("CARGO_PKG_NAME"),
                            env!("CARGO_PKG_VERSION")
                        ));

                    let listener = match listener::Listener::new(
                        // Using app's config for testing
                        ListenerConfig {
                            poll_interval: app.cfg.poll_interval,
                            channel_url: url.clone(),
                            proxy_list_url: app.cfg.proxy_list_url.clone(),
                            webhook_url: app.cfg.webhook_url.clone(),
                            webhook_secret: app.cfg.webhook_secret.clone(),
                        },
                        app.db.clone(),
                        client_builder,
                    )
                    .await
                    {
                        Ok(listener) => Arc::new(listener),
                        Err(e) => {
                            tracing::error!("failed to create listener for {}: {e}", url);
                            continue;
                        }
                    };

                    self.listeners
                        .lock()
                        .await
                        .insert(url.clone(), Arc::clone(&listener));

                    set.spawn_local(async move { listener.run().await });
                }

                tokio::select! {
                    _ = self.shutdown.cancelled() => {
                        tracing::info!("shutting down all listeners");
                        for (url, listener) in self.listeners.lock().await.clone() {
                            if let Err(e) = listener.stop().await {
                                tracing::error!("failed to stop listener for {}: {e}", url);
                            }
                        }
                    }
                    _ = async {
                        while let Some(res) = set.join_next().await {
                            match res {
                                Ok(Err(e)) => tracing::error!("listener error: {e}"),
                                Err(e) => tracing::error!("listener panicked: {e}"),
                                Ok(Ok(())) => {}
                            }
                        }
                    } => {
                        tracing::warn!("all listeners have stopped");
                    }
                }
            })
            .await;
        Ok(())
    }
}
