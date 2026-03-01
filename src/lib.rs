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

pub mod api;
pub mod config;
mod db;
pub mod listener;
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
        tracing::info!("adding {} listeners", self.cfg.channels.len());
        // Local set is needed because scraper is !Send
        let local = tokio::task::LocalSet::new();

        local
            .run_until(async {
                for id in &self.cfg.channels {
                    let listeners = self.db.get_all_listeners().await.unwrap();
                        for listener in listeners {
                            if let Err(e) = self.add_listener(listener.into()).await {
                                tracing::error!("failed to add listener {}: {e}", id);
                        }
                    }
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
                                // If channel closed shutdown the server
                                None => self.shutdown.cancel(),
                            }
                        }
                    }
                }
            })
            .await;

        Ok(())
    }

    /// Send an add command to server to create a [Listener].
    pub async fn add_listener(&self, cfg: ListenerConfig) -> anyhow::Result<()> {
        // Add to db
        if let Err(e) = self.db.insert_listener(cfg.clone().into()).await {
            tracing::error!("failed to add listener to db: {e}");
        }

        self.cmd_tx.send(ListenerCmd::Add(cfg)).await?;
        Ok(())
    }

    /// Send a remove command to server to remove a [Listener]
    pub async fn remove_listener(&self, id: &str) -> anyhow::Result<()> {
        self.cmd_tx
            .send(ListenerCmd::Remove(id.to_string()))
            .await?;

        // Remove from db
        if let Err(e) = self.db.delete_listener(id).await {
            tracing::error!("failed to delete listener from db {id}: {e}");
        }
        Ok(())
    }

    /// Update a [Listener]
    ///
    /// Works by calling [Server::remove_listener] and then [Server::add_listener].
    /// Maybe can be improved in the future.
    pub async fn update_listener(&self, cfg: ListenerConfig) -> anyhow::Result<()> {
        self.remove_listener(&cfg.id).await?;
        self.add_listener(cfg).await?;
        Ok(())
    }

    /// Get a [Listener] by id from the database
    pub async fn get_listener(&self, id: &str) -> anyhow::Result<Option<model::ListenerRow>> {
        self.db.get_listener(id).await
    }

    /// Get all [Listener]s from the database
    pub async fn get_all_listeners(&self) -> anyhow::Result<Vec<model::ListenerRow>> {
        self.db.get_all_listeners().await
    }

    /// Stop all [Listener]s and clear the listeners hashmap.
    async fn stop_all(&self) {
        tracing::info!("stopping all listeners");

        let listeners = {
            let locked = self.listeners.lock().await;
            locked.values().cloned().collect::<Vec<_>>()
        };

        for listener in listeners {
            self.shutdown_listener(&listener.cfg.id).await;
        }
    }

    async fn spawn_listener(&self, cfg: ListenerConfig) {
        // Check if listenr already exists
        if self.listeners.lock().await.contains_key(&cfg.id) {
            tracing::warn!("listener {} already exists", cfg.id);
            return;
        }

        let listener = match Listener::new(cfg, self.db.clone()).await {
            Ok(l) => Arc::new(l),
            Err(e) => {
                tracing::error!("failed to create listener: {e}");
                return;
            }
        };

        // Add to listeners map
        self.listeners
            .lock()
            .await
            .insert(listener.cfg.id.clone(), Arc::clone(&listener));

        // Start listener
        tokio::task::spawn_local({
            let listener = Arc::clone(&listener);
            async move { listener.run().await }
        });
    }

    async fn shutdown_listener(&self, id: &str) {
        // Remove from listeners map
        let listener = {
            let mut listeners = self.listeners.lock().await;
            listeners.remove(id)
        };

        // Stop listener
        if let Some(listener) = listener {
            if let Err(e) = listener.stop().await {
                tracing::error!("failed to stop listener {id}: {e}");
            }
        } else {
            tracing::warn!("listener not found for channel {}", id);
        }
    }
}
