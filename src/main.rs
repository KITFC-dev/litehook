use tracing_subscriber;
use tokio;
use std::time::Duration;
use anyhow::Result;
use std::fs;
use std::path::Path;

mod model;
mod config;
mod web;
mod db;
use db::Db;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("loading config");
    let cfg = config::load()?;

    let db = init_db()?;

    tracing::info!("listening to {}", cfg.telegram.url);
    loop {
        if let Err(e) = run_cycle(&cfg.telegram.url, &db).await {
            tracing::error!("{e}");
        }

        tokio::time::sleep(Duration::from_secs(cfg.server.poll_interval_seconds)).await;
    }
}

async fn run_cycle(channel: &str, db: &Db) -> Result<()> {
    let html = web::fetch_html(channel).await?;
    let posts = web::parse_posts(&html).await?;

    for post in &posts {
        let p = db.get_posts(&post.id)?;
        if !p.is_some() {
            db.insert_post(post)?;
            tracing::info!("new post: {}", post.id);
        }
    }

    Ok(())
}

fn init_db() -> Result<Db> {
    tracing::info!("initializing database");
    fs::create_dir_all(Path::new("data"))?;
    let db = Db::new("data/posts.db")?;

    Ok(db)
}
