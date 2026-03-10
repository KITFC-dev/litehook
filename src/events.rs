use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub struct EventHandler {
    rx: mpsc::Receiver<String>,
    shutdown: CancellationToken
}

impl EventHandler {
    pub fn new(rx: mpsc::Receiver<String>) -> Self {
        Self {
            rx,
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
                Some(msg) = self.rx.recv() => {
                    tracing::info!("received new event: {}", msg);
                }
            }
        }
    }

    pub async fn stop(mut self) {
        self.shutdown.cancel();
        self.rx.close();
    }
}
