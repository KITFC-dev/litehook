use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::model::{Page, Post};

/// Event
#[derive(Debug)]
pub enum Event {
    NewPost(Post),
    Scrape(Page),
    Test(String),
}

pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
    shutdown: CancellationToken,
}

impl EventHandler {
    pub fn new(rx: mpsc::Receiver<Event>) -> Self {
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
                Some(event) = self.rx.recv() => {
                    self.handle_event(event).await
                }
            }
        }
    }

    pub async fn handle_event(&mut self, event: Event) {
        tracing::info!("received new event: {:#?}", event);
    }

    pub async fn stop(mut self) {
        self.shutdown.cancel();
        self.rx.close();
    }
}
