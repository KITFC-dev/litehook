use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;

use crate::events::Event;
use crate::sources::{Source, SourceConfig};

pub struct SourceRegistration {
    pub kind:    &'static str,
    pub factory: fn(SourceConfig, mpsc::Sender<Event>) -> Pin<Box<dyn Future<Output = anyhow::Result<Box<dyn Source + Send>>> + Send>>,
}

inventory::collect!(SourceRegistration);

/// Build a source from config
pub async fn build(cfg: SourceConfig, tx: mpsc::Sender<Event>) -> anyhow::Result<Box<dyn Source + Send>> {
    let registration = inventory::iter::<SourceRegistration>()
        .find(|r| r.kind == cfg.kind)
        .ok_or_else(|| anyhow::anyhow!("no source registered for kind `{}`", cfg.kind))?;

    (registration.factory)(cfg, tx).await
}
