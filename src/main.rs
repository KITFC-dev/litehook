use anyhow::{Result, Ok};
use litehook::{App, config};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_level(true)
        .with_target(false)
        .init();
    let cfg = config::Config::from_dotenv()?;
    let app = App::new(cfg).await?;

    app.run().await?;

    Ok(())
}
