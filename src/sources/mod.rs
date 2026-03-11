use serde::{Deserialize, Serialize};
use sqlx::FromRow;

pub mod registry;
pub mod telegram;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SourceConfig {
    pub id: String,
    pub kind: String,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub id: String,
    pub kind: String,
    pub raw: serde_json::Value,
    pub active: bool,
}

impl From<SourceConfig> for SourceInfo {
    fn from(cfg: SourceConfig) -> Self {
        Self {
            id: cfg.id,
            kind: cfg.kind,
            raw: cfg.raw,
            active: false,
        }
    }
}

/// Source trait
#[async_trait::async_trait]
pub trait Source: Send + Sync {
    /// Get the id of the source
    fn id(&self) -> &str;

    /// Source Name
    fn name(&self) -> &'static str;

    /// Run the source
    async fn run(&self) -> anyhow::Result<()>;

    /// Stop the source
    async fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
