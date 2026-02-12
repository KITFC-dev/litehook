use tracing_subscriber;
use tokio;
use std::time::Duration;
use anyhow::Result;
use std::fs;
use std::path::Path;
use dotenvy::dotenv;
use std::env;

mod model;
mod web;
mod db;
use db::Db;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("loading config");
    dotenv().ok();

    let db = init_db()?;

    tracing::info!("started listening to {}", &env::var("CHANNEL_URL").expect("CHANNEL_URL not set"));
    loop {
        if let Err(e) = run_cycle(&db).await {
            tracing::error!("{e}");
        }
        
        let interval = env::var("POLL_INTERVAL")
            .expect("POLL_INTERVAL not set")
            .parse::<u64>()
            .expect("POLL_INTERVAL must be a valid u64");
        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
}

async fn run_cycle(db: &Db) -> Result<()> {
    let html = web::fetch_html().await?;
    let posts = web::parse_posts(&html).await?;

    for post in &posts {
        let p = db.get_posts(&post.id)?;
        if !p.is_some() {
            tracing::info!("new post: {}", post.id);
            db.insert_post(post)?;
            web::send_webhook(post).await?;
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
