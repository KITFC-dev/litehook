use anyhow::{Ok, Result};
use litehook::{Server, api::Api, config};
use tracing_subscriber::fmt::time::ChronoLocal;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S".to_string()))
        .with_max_level(tracing::Level::INFO)
        .with_level(true)
        .with_target(false)
        .init();

    let cfg = config::Config::from_dotenv()?;
    let server = std::sync::Arc::new(Server::new(cfg.clone()).await?);

    let shutdown_handle = tokio::spawn({
        let shutdown_token = server.shutdown.clone();
        async move {
            handle_signal().await;
            shutdown_token.cancel();
        }
    });

    let web_api = tokio::spawn({
        let cfg = cfg.clone();
        let server = std::sync::Arc::clone(&server);
        async move {
            let api = Api::new(cfg, server).await.unwrap();
            api.run().await.unwrap();
        }
    });

    server.run().await.unwrap();
    web_api.await.unwrap();
    shutdown_handle.await.unwrap();

    tracing::info!("bye!");
    Ok(())
}

pub async fn handle_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {},
            _ = sigint.recv() => {},
        }
    }

    #[cfg(windows)]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    }

    tracing::info!("received shutdown signal");
}
