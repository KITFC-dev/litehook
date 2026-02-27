use std::sync::Arc;
use axum::{
    Router, Json,
    extract::{State, Path},
    http::StatusCode,
    routing::{delete, get, post, put},
};

use crate::{Server};
use crate::config::{Config, ListenerConfig};

pub struct Api {
    cfg: Config,
    router: Router,
    server: Arc<Server>,
}

impl Api {
    pub async fn new(cfg: Config, server: Arc<Server>) -> anyhow::Result<Self> {
        tracing::info!("starting web api");
        let router = Router::new()
            .route("/listeners", get(Self::get_all_listeners))
            .route("/listeners", post(Self::add_listener))
            .route("/listeners/{id}", get(Self::get_listener))
            .route("/listeners/{id}", put(Self::update_listener))
            .route("/listeners/{id}", delete(Self::remove_listener))
            .with_state(Arc::clone(&server));
        Ok(Self {
            cfg,
            router,
            server,
        })
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.cfg.port)).await?;

        axum::serve(listener, self.router.clone())
            .with_graceful_shutdown(self.server.shutdown.clone().cancelled_owned())
            .await?;

        tracing::info!("web api stopped");
        Ok(())
    }

    async fn get_all_listeners(
        State(_server): State<Arc<Server>>
    ) -> (StatusCode, Json<Vec<String>>)
    {
        //! Return empty string vec for testing
        (StatusCode::OK, Json(vec![]))
    }

    async fn add_listener(
        State(server): State<Arc<Server>>,
        Json(body): Json<ListenerConfig>
    ) -> StatusCode
    {
        server.add_listener(body).await;
        StatusCode::OK
    }

    async fn get_listener(
        State(_server): State<Arc<Server>>,
        Path(id): Path<String>
    ) -> (StatusCode, String) {
        //! Return empty string for testing
        (StatusCode::OK, id.to_string())
    }

    async fn update_listener(
        State(server): State<Arc<Server>>,
        Json(body): Json<ListenerConfig>
    ) -> StatusCode {
        server.update_listener(body).await;
        StatusCode::OK
    }

    async fn remove_listener(
        State(server): State<Arc<Server>>,
        Path(id): Path<String>
    ) -> StatusCode {
        server.remove_listener(&id).await;
        StatusCode::OK
    }
}
