use tracing_subscriber;
use tokio;
use std::time::Duration;
use anyhow::Result;

mod model;
mod config;
mod web;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = config::load()?;
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("webhook url: {}", cfg.webhook.url);
    loop {
        if let Err(e) = run_cycle(&cfg.telegram.url).await {
            tracing::error!("{e}");
        }

        tokio::time::sleep(Duration::from_secs(cfg.server.poll_interval_seconds)).await;
        break;
    }

    Ok(())
}

async fn run_cycle(channel: &str) -> Result<()> {
    let html = web::fetch_html(channel).await?;
    // tracing::info!("{}", html.to_string());

    for post in web::parse_posts(&html).await? {
        tracing::info!("{:?}", post.id);
    }

    tracing::info!("{} posts parsed", web::parse_posts(&html).await?.len());
    Ok(())
}
