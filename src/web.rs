use anyhow::Result;
use reqwest::Client;
use std::time::Duration;

pub async fn fetch_html(url: &str) -> Result<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("litehook/0.1")
        .build()?;

    let html = client.get(url).send().await?.text().await?;
    Ok(html)
}
