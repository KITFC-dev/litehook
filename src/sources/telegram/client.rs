use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::TelegramClientConfig;
use crate::Arc;

pub struct TelegramClient {
    pub cfg: Arc<RwLock<TelegramClientConfig>>,

    tx: mpsc::Sender<String>,

    shutdown: CancellationToken,
}

impl TelegramClient {
    pub async fn new(cfg: TelegramClientConfig, tx: mpsc::Sender<String>) -> anyhow::Result<Self> {
        tracing::info!("initializing listener {}", cfg.id);
        Ok(Self {
            cfg: Arc::new(RwLock::new(cfg)),
            tx,
            shutdown: CancellationToken::new(),
        })
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let channel_ids: Vec<i64> = {
            let cfg = self.cfg.read().await;
            cfg.channels.iter().map(|c| c.id).collect()
        };
        loop {
            tokio::select! {
                // Shutdown handler
                _ = self.shutdown.cancelled() => {
                    self.stop().await?;
                    return Ok(());
                }

                res = self.start_listener(channel_ids.clone()) => { res? }
            }
        }
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        let id = self.cfg.read().await.id.clone();
        tracing::info!("stopping listener {}", id);
        self.shutdown.cancel();
        Ok(())
    }

    pub async fn start_listener(&self, channel_ids: Vec<i64>) -> anyhow::Result<()> {
        tracing::info!("starting listening to channels: {:#?}", channel_ids);
        
        sleep(Duration::from_secs(3)).await;

        self.tx.send("test".to_string()).await.unwrap();
        Ok(())
    }
}
