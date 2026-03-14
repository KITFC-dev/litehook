use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use super::TelegramClientConfig;
use crate::Arc;
use crate::events::Event;

pub struct TelegramClient {
    pub cfg: Arc<RwLock<TelegramClientConfig>>,

    #[allow(unused)]
    tx: mpsc::Sender<Event>,

    shutdown: CancellationToken,
}

impl TelegramClient {
    pub async fn new(cfg: TelegramClientConfig, tx: mpsc::Sender<Event>) -> anyhow::Result<Self> {
        tracing::info!("initializing listener {}", cfg.id);
        Ok(Self {
            cfg: Arc::new(RwLock::new(cfg)),
            tx,
            shutdown: CancellationToken::new(),
        })
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let channel_ids = self.cfg.read().await.channels.clone();
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
        tracing::info!("stopping listener with id {}", id);
        self.shutdown.cancel();
        Ok(())
    }

    pub async fn start_listener(&self, channel_ids: Vec<String>) -> anyhow::Result<()> {
        tracing::info!("starting listening to channels: {:#?}", channel_ids);

        let (ntf_tx, ntf_rx) = oneshot::channel();

        self.tx
            .send(Event::InputRequest(
                "This is test, please reply!!".to_string(),
                ntf_tx,
            ))
            .await?;

        tracing::info!("Recieved: {}", ntf_rx.await?);

        self.stop().await?;
        Ok(())
    }
}
