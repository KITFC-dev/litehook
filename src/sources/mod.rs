pub mod telegram;

pub struct SourceConfig {
    pub kind: String,
    pub raw: serde_json::Value,
}

/// Source trait
#[async_trait::async_trait(?Send)]
pub trait Source {
    /// Source Name
    fn name(&self) -> &'static str;

    /// Run the source
    async fn run(&self) -> anyhow::Result<()>;

    /// Stop the source
    async fn stop(&self) {}
}
