use axum::{
    routing::{get, post, put, delete}, 
    Router
};

use crate::config::Config;
use crate::Server;

#[allow(unused)]
pub struct Api {
    cfg: Config,
    router: Router,
    server: std::sync::Arc<Server>
}

impl Api {
    #[allow(unused)]
    pub async fn new(cfg: Config, server: std::sync::Arc<Server>) -> anyhow::Result<Self> {
        tracing::info!("starting web dashboard");
        let router = Router::new()
            .route("/listeners", get(Self::get_all_listeners))
            .route("/listeners", post(Self::add_listener))
            .route("/listeners/{id}", get(Self::get_listener))
            .route("/listeners/{id}", put(Self::update_listener))
            .route("/listeners/{id}", delete(Self::remove_listener))
            .with_state(std::sync::Arc::clone(&server));
        Ok(Self { cfg, router, server })
    }

    #[allow(unused)]
    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.cfg.port)).await?;
        
        axum::serve(listener, self.router.clone())
            .with_graceful_shutdown(self.server.shutdown.clone().cancelled_owned())
            .await?;

        tracing::info!("web dashboard stopped");
        Ok(())
    }

    async fn get_all_listeners() {
        unimplemented!()
    }

    async fn add_listener() {
        unimplemented!()
    }

    async fn get_listener() {
        unimplemented!()
    }

    async fn update_listener() {
        unimplemented!()
    }

    async fn remove_listener() {
        unimplemented!()
    }
}
