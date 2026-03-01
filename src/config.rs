use serde::Deserialize;
use url::Url;

/// Litehook server configuration
#[derive(Debug, Deserialize, Clone)]
pub struct EnvConfig {
    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_db_path")]
    pub db_path: String,

    pub channels: Option<Vec<String>>,
}

/// Global listener configuration
#[derive(Debug, Deserialize, Clone)]
pub struct GlobalListenerConfig {
    #[serde(default = "default_interval")]
    pub poll_interval: Option<i64>,

    pub webhook_url: Option<String>,
    pub proxy_list_url: Option<String>,
    pub webhook_secret: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ListenerConfig {
    pub id: String,

    #[serde(default = "default_interval")]
    pub poll_interval: Option<i64>,
    pub channel_url: String,
    pub proxy_list_url: Option<String>,
    pub webhook_url: Option<String>,
    pub webhook_secret: Option<String>,
}

impl EnvConfig {
    pub fn from_dotenv() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();
        Ok(envy::from_env()?)
    }

    pub fn validate(&self, global_cfg: &GlobalListenerConfig) -> anyhow::Result<()> {
        if self.channels.is_some() && global_cfg.webhook_url.is_none() {
            anyhow::bail!("WEBHOOK_URL is required when CHANNELS is set");
        }
        Ok(())
    }
}

impl GlobalListenerConfig {
    pub fn from_dotenv() -> anyhow::Result<Self> {
        Ok(envy::from_env()?)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(webhook_url) = &self.webhook_url {
            Url::parse(webhook_url)
                .map_err(|_| anyhow::anyhow!("webhook_url is not a valid URL: {}", webhook_url))?;
        }

        if let Some(proxy_url) = &self.proxy_list_url {
            Url::parse(proxy_url)
                .map_err(|_| anyhow::anyhow!("proxy_list_url is not a valid URL: {}", proxy_url))?;
        }

        if self.poll_interval.unwrap_or(default_interval().unwrap()) <= 2 {
            anyhow::bail!("poll_interval must be at least 2 seconds");
        }

        Ok(())
    }
}

impl ListenerConfig {
    /// Merge values from [EnvConfig]
    pub fn merge_with(mut self, cfg: &GlobalListenerConfig) -> Self {
        if self.proxy_list_url.is_none() || self.proxy_list_url.as_deref() == Some("") {
            self.proxy_list_url = cfg.proxy_list_url.clone();
        }
        if self.webhook_secret.is_none() || self.webhook_secret.as_deref() == Some("") {
            self.webhook_secret = cfg.webhook_secret.clone();
        }
        if self.poll_interval.is_none() {
            self.poll_interval = cfg.poll_interval;
        }
        if self.webhook_url.is_none() || self.webhook_url.as_deref() == Some("") {
            self.webhook_url = cfg.webhook_url.clone();
        }

        self
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if !self.channel_url.starts_with("https://t.me/s/") {
            anyhow::bail!(
                "channel_url must start with https://t.me/s/: {}",
                self.channel_url
            );
        }

        match &self.webhook_url {
            Some(url) => {
                Url::parse(url)
                    .map_err(|_| anyhow::anyhow!("webhook_url is not a valid URL: {}", url))?;
            }
            None => anyhow::bail!("webhook_url is required for listener {}", self.id),
        }

        if self.poll_interval.unwrap_or(default_interval().unwrap()) <= 2 {
            anyhow::bail!(
                "poll_interval must be at least 2 seconds for listener {}",
                self.id
            );
        }

        Ok(())
    }
}

fn default_port() -> u16 {
    4101
}

fn default_interval() -> Option<i64> {
    Some(600)
}

fn default_db_path() -> String {
    "data/litehook.db".to_string()
}
