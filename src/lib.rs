//! litehook
//!
//! Polls a public Telegram channel page and sends webhook notifications
//! when new posts are detected. State is stored in SQLite database.

use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, mpsc, watch};
use tokio_util::sync::CancellationToken;

use config::{EnvConfig, GlobalListenerConfig};
use events::{Event, EventHandler};

use crate::sources::registry;
use crate::sources::{Source, SourceConfig, SourceInfo};

pub mod api;
pub mod config;
pub mod db;
pub mod events;
pub mod model;
pub mod sources;

/// Core server state for the Litehook server.
///
/// Holds configuration, database connection, HTTP client,
/// and shutdown signal.
pub struct Server {
    /// Tokio Cancellation token for shutdown signal
    pub shutdown: CancellationToken,

    sources: Mutex<HashMap<String, Arc<Box<dyn Source + Send>>>>,
    #[allow(unused)]
    env: EnvConfig,
    db: db::Db,

    cmd_tx: mpsc::Sender<SourceCmd>,
    cmd_rx: Mutex<Option<mpsc::Receiver<SourceCmd>>>,
    cfg_tx: watch::Sender<GlobalListenerConfig>,
    event_tx: mpsc::Sender<Event>,
    event_rx: Mutex<Option<mpsc::Receiver<Event>>>,
}

/// Commands for the [Server] to manage sources
pub enum SourceCmd {
    Add(SourceConfig),
    Remove(String),
}

impl Server {
    /// Create a new instance of [Server].
    ///
    /// Creates SQLite database in data/litehook.db and creates data dir
    /// if it doesn't exist. HTTP client is configured with a 10 second timeout.
    pub async fn new() -> anyhow::Result<Self> {
        tracing::info!("initializing");
        let env = EnvConfig::from_dotenv()?;
        let (cmd_tx, cmd_rx) = mpsc::channel(100);
        let global_cfg = GlobalListenerConfig::from_dotenv()?;
        global_cfg.validate()?;
        env.validate(&global_cfg)?;
        let (cfg_tx, _) = watch::channel(global_cfg);
        let (event_tx, event_rx) = mpsc::channel(100);

        let db = db::Db::new(&env.db_path).await?;

        Ok(Self {
            shutdown: CancellationToken::new(),
            sources: Mutex::new(HashMap::new()),
            env,
            db,
            cmd_tx,
            cmd_rx: Mutex::new(Some(cmd_rx)),
            cfg_tx,
            event_tx,
            event_rx: Mutex::new(Some(event_rx)),
        })
    }

    /// Run [Server]
    ///
    /// Spawns listener local tasks listens to mpsc commands
    /// and handles shutdown signal.
    pub async fn run(self: Arc<Self>) -> anyhow::Result<()> {
        // Start event handler
        let event_rx = self
            .event_rx
            .lock()
            .await
            .take()
            .expect("event receiver already taken");
        let event_handler = EventHandler::new(event_rx);
        tokio::spawn(async move { event_handler.run().await });

        // Load sources from db
        for cfg in self.db.get_all_sources().await? {
            self.spawn_source(&cfg).await;
        }

        // Command loop
        let mut cmd_rx = self
            .cmd_rx
            .lock()
            .await
            .take()
            .expect("cmd receiver already taken");

        loop {
            tokio::select! {
                _ = self.shutdown.cancelled() => {
                    self.stop_all().await;
                    break;
                }
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(SourceCmd::Add(cfg))    => self.spawn_source(&cfg).await,
                        Some(SourceCmd::Remove(id))  => self.shutdown_source(&id).await,
                        None                         => self.shutdown.cancel(),
                    }
                }
            }
        }

        Ok(())
    }

    /// Send a command to create a [Source].
    pub async fn add_source(&self, cfg: &SourceConfig) -> anyhow::Result<()> {
        // TODO
        // let cfg = cfg.merge_with(&self.cfg_tx.borrow());
        // cfg.validate()?; // validate before sending command

        self.db.insert_source(cfg).await?;
        self.cmd_tx.send(SourceCmd::Add(cfg.clone())).await?;

        Ok(())
    }

    /// Send a command to remove a [Source].
    pub async fn remove_source(&self, id: &str) -> anyhow::Result<()> {
        self.cmd_tx.send(SourceCmd::Remove(id.to_string())).await?;

        // Remove from db
        if let Err(e) = self.db.delete_source(id).await {
            tracing::error!("failed to delete source from the db {id}: {e}");
        }

        Ok(())
    }

    /// Update [Source] with a new [SourceConfig] and [EnvConfig].
    pub async fn update_source(&self, cfg: &SourceConfig) -> anyhow::Result<()> {
        let source = self
            .sources
            .lock()
            .await
            .get(&cfg.id)
            .ok_or(anyhow::anyhow!("source not found"))?
            .clone();

        let _global_cfg = self.cfg_tx.borrow().clone();
        self.shutdown_source(source.id()).await;
        //TODO: merge with global config
        self.spawn_source(cfg).await;

        self.db.insert_source(&cfg).await?;

        Ok(())
    }

    /// Check if the [Source] is running.
    pub async fn check_source_running(&self, id: &str) -> bool {
        let sources = self.sources.lock().await;
        sources.contains_key(id)
    }

    /// Get a [Source] by id from the database
    pub async fn get_source(&self, id: &str) -> anyhow::Result<Option<SourceInfo>> {
        let mut res: SourceInfo = match self.db.get_source(id).await? {
            Some(r) => r.into(),
            None => return Ok(None),
        };

        res.active = self.check_source_running(id).await;
        Ok(Some(res))
    }

    /// Get all [Source]s from the database.
    pub async fn get_all_sources(&self) -> anyhow::Result<Vec<SourceInfo>> {
        let running = self.sources.lock().await;

        let sources = self
            .db
            .get_all_sources()
            .await?
            .into_iter()
            .map(|cfg| {
                let active = running.contains_key(&cfg.id);
                let mut info = SourceInfo::from(cfg);
                info.active = active;
                info
            })
            .collect();

        Ok(sources)
    }

    pub async fn health(&self) -> anyhow::Result<model::Health> {
        let sources = self.sources.lock().await;
        Ok(model::Health {
            ok: true,
            sources: sources.len(),
        })
    }

    /// Shutdowns all [Source]s.
    async fn stop_all(&self) {
        tracing::info!("stopping all sources");

        let sources = {
            let locked = self.sources.lock().await;
            locked.values().cloned().collect::<Vec<_>>()
        };

        for s in sources {
            self.shutdown_source(s.id()).await;
        }
    }

    async fn spawn_source(&self, cfg: &SourceConfig) {
        // Check if source already exists
        if self.sources.lock().await.contains_key(&cfg.id) {
            tracing::warn!("source with id '{}' already exists", cfg.id);
            return;
        }

        // Build source
        let id = cfg.id.clone();
        let source = match registry::build(cfg.clone(), self.event_tx.clone()).await {
            Ok(s) => Arc::new(s),
            Err(e) => {
                tracing::error!("failed to build source: {e}");
                return;
            }
        };

        self.sources
            .lock()
            .await
            .insert(id.clone(), Arc::clone(&source));

        // Spawn source
        tokio::task::spawn_local(async move {
            if let Err(e) = source.run().await {
                tracing::error!("source {id} error: {e}");
            }
        });
    }

    async fn shutdown_source(&self, id: &str) {
        // Remove from sources map
        let source = {
            let mut sources = self.sources.lock().await;
            sources.remove(id)
        };

        // Stop source
        if let Some(source) = source {
            if let Err(e) = source.stop().await {
                tracing::error!("failed to stop source {id}: {e}");
            }
        } else {
            tracing::warn!("source not found for id {}", id);
        }
    }
}
