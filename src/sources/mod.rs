use tokio::sync::mpsc;

pub mod telegram;

/// Source trait
#[async_trait::async_trait]
pub trait Source: Send + Sync {
    /// Source Name
    fn name(&self) -> &'static str;

    /// Run the source
    async fn run(&self, tx: mpsc::Sender<String>) -> anyhow::Result<()>;

    /// Stop the source
    async fn stop(&self) {}
}
