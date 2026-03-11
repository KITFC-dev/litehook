use serde::{Deserialize, Serialize};
use sqlx::FromRow;

pub mod telegram;
pub mod registry;

#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct SourceConfig {
    pub id: String,
    pub kind: String,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceInfo {
    pub id:     String,
    pub kind:   String,
    pub raw:    serde_json::Value,
    pub active: bool,
}

impl From<SourceConfig> for SourceInfo {
    fn from(cfg: SourceConfig) -> Self {
        Self {
            id:     cfg.id,
            kind:   cfg.kind,
            raw:    cfg.raw,
            active: false,
        }
    }
}

/// Source trait
#[async_trait::async_trait(?Send)]
pub trait Source {
    /// Get the id of the source
    fn id(&self) -> &str;

    /// Source Name
    fn name(&self) -> &'static str;

    /// Run the source
    async fn run(&self) -> anyhow::Result<()>;

    /// Stop the source
    async fn stop(&self) -> anyhow::Result<()> { Ok(()) }
}
