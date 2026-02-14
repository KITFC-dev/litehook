use tracing_subscriber::fmt::time::ChronoLocal;
use anyhow::{Result, Ok};
use litehook::{App, config};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S".to_string()))
        .with_max_level(tracing::Level::INFO)
        .with_level(true)
        .with_target(false)
        .init();

    let cfg = config::Config::from_dotenv()?;
    let app = App::new(cfg).await?;

    let shutdown = tokio::spawn({
        let shutdown_token = app.shutdown.clone();
        async move {
            tokio::signal::ctrl_c().await.unwrap();
            tracing::info!("shutting down...");
            shutdown_token.cancel();
        }
    });

    let res = app.run().await;
    if let Err(e) = res {
        tracing::error!("app failed: {e}");
    }

    shutdown.await.unwrap();

    tracing::info!("bye!");
    Ok(())
}
