use serde::Deserialize;
use std::sync::OnceLock;

pub static ENV: OnceLock<EnvConfig> = OnceLock::new();

pub fn init_env(cfg: EnvConfig) {
    ENV.set(cfg).expect("environment already initialized");
}

pub fn get_env() -> &'static EnvConfig {
    ENV.get().expect("environment not initialized")
}

/// Litehook server configuration
#[derive(Debug, Deserialize, Clone)]
pub struct EnvConfig {
    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_db_path")]
    pub db_path: String,

    pub webhook_secret: Option<String>,
    pub proxy_list_url: Option<String>,
}

impl EnvConfig {
    pub fn from_dotenv() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();
        Ok(envy::from_env()?)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.webhook_secret.is_none() {
            tracing::warn!("webhook_secret is not set");
        }
        Ok(())
    }
}

fn default_port() -> u16 {
    4101
}

fn default_db_path() -> String {
    "data/litehook.db".to_string()
}
