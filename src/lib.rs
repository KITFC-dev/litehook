//! litehook
//!
//! Polls a public Telegram channel page and sends webhook notifications
//! when new posts are detected. State is stored in SQLite database.

use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;

use config::{Config, ListenerConfig};
use db::Db;
use listener::Listener;

pub mod config;
mod db;
mod listener;
mod model;
mod parser;

/// Core server state for the Litehook server.
///
/// Holds configuration, database connection, HTTP client,
/// and shutdown signal.
pub struct Server {
    /// Tokio Cancellation token for shutdown signal
    pub shutdown: CancellationToken,

    listeners: Mutex<HashMap<String, Arc<Listener>>>,
    cfg: Config,
    db: Db,

    cmd_tx: mpsc::Sender<ListenerCmd>,
    cmd_rx: Mutex<mpsc::Receiver<ListenerCmd>>,
}

/// Commands for the [Server] to manage listeners
pub enum ListenerCmd {
    Add(ListenerConfig),
    Remove(String),
}

impl Server {
    /// Create a new instance of [Server].
    ///
    /// Creates SQLite database in data/litehook.db and creates data dir
    /// if it doesn't exist. HTTP client is configured with a 10 second timeout.
    pub async fn new(cfg: Config) -> anyhow::Result<Self> {
        tracing::info!("initializing");
        let (cmd_tx, cmd_rx) = mpsc::channel(100);
        let db = Db::new(&cfg.db_path).await?;

        Ok(Self {
            shutdown: CancellationToken::new(),
            listeners: Mutex::new(HashMap::new()),
            cfg,
            db,
            cmd_tx,
            cmd_rx: Mutex::new(cmd_rx),
        })
    }

    /// Run [Server]
    ///
    /// Spawns listener local tasks listens to mpsc commands
    /// and handles shutdown signal.
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

                let mut cmd_rx = self.cmd_rx.lock().await;
                loop {
                    tokio::select! {
                        _ = self.shutdown.cancelled() => {
                            self.stop_all().await;
                            break;
                        }
                        cmd = cmd_rx.recv() => {
                            match cmd {
                                Some(ListenerCmd::Add(cfg)) => self.spawn_listener(cfg).await,
                                Some(ListenerCmd::Remove(id)) => self.shutdown_listener(&id).await,
                                None => break, // Channel closed
                            }
                        }
                    }
                }
            })
            .await;

        Ok(())
    }

    /// Stop all [Listener]s and clear the listeners hashmap.
    async fn stop_all(&self) {
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
        self.cmd_tx.send(ListenerCmd::Add(cfg)).await.unwrap();
    }

    /// Remove a [Listener] from the server.
    pub async fn remove_listener(&self, id: &str) {
        self.cmd_tx
            .send(ListenerCmd::Remove(id.to_string()))
            .await
            .unwrap();
    }

    /// Update a [Listener]
    ///
    /// Works by removing the old listener and adding a new one
    /// with the updated configuration. Maybe can be improved in the future.
    #[allow(unused)]
    async fn update_listener(&self, cfg: ListenerConfig) {
        self.remove_listener(&cfg.id).await;
        self.add_listener(cfg).await;
    }

    /// Get a [Listener] by id
    pub async fn get_listener(&self, id: &str) -> Option<Arc<Listener>> {
        let listeners = self.listeners.lock().await;
        listeners.get(id).cloned()
    }

    /// Get all [Listener]s
    pub async fn get_all_listeners(&self) -> Vec<Arc<Listener>> {
        let listeners = self.listeners.lock().await;
        listeners.values().cloned().collect()
    }

    async fn spawn_listener(&self, cfg: ListenerConfig) {
        let id = cfg.id.clone();
        let listener = match Listener::new(cfg, self.db.clone()).await {
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

    async fn shutdown_listener(&self, id: &str) {
        let mut listeners = self.listeners.lock().await;
        if let Some(listener) = listeners.remove(id) {
            if let Err(e) = listener.stop().await {
                tracing::error!("failed to stop listener: {e}");
            }
        } else {
            tracing::warn!("listener not found for channel {}", id);
        }
    }
}
