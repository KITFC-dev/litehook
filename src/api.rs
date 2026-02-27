use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
};
use std::sync::Arc;

use crate::config::{Config, ListenerConfig};
use crate::{Server, model::ListenerResponse};

/// Web API and dashboard for managing [Server] listeners.
///
/// ### REST Endpoints
///
/// | Method | Path | Handler |
/// |--------|------|---------|
/// | `GET` | `/listeners` | [get_all_listeners] |
/// | `POST` | `/listeners` | [add_listener] |
/// | `GET` | `/listeners/{id}` | [get_listener] |
/// | `PUT` | `/listeners/{id}` | [update_listener] |
/// | `DELETE` | `/listeners/{id}` | [remove_listener] |
pub struct Api {
    cfg: Config,
    router: Router,
    server: Arc<Server>,
}

impl Api {
    /// Create a new instance of [Api]
    pub async fn new(cfg: Config, server: Arc<Server>) -> anyhow::Result<Self> {
        tracing::info!("starting web api");
        let router = Router::new()
            .route("/listeners", get(get_all_listeners))
            .route("/listeners", post(add_listener))
            .route("/listeners/{id}", get(get_listener))
            .route("/listeners/{id}", put(update_listener))
            .route("/listeners/{id}", delete(remove_listener))
            .with_state(Arc::clone(&server));
        Ok(Self {
            cfg,
            router,
            server,
        })
    }

    /// Run [Api]
    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.cfg.port)).await?;

        axum::serve(listener, self.router.clone())
            .with_graceful_shutdown(self.server.shutdown.clone().cancelled_owned())
            .await?;

        tracing::info!("web api stopped");
        Ok(())
    }
}

pub async fn get_all_listeners(
    State(server): State<Arc<Server>>,
) -> (StatusCode, Json<Vec<ListenerResponse>>) {
    let listeners = server.get_all_listeners().await;
    (StatusCode::OK, Json(listeners))
}

pub async fn add_listener(
    State(server): State<Arc<Server>>,
    Json(body): Json<ListenerConfig>,
) -> StatusCode {
    server.add_listener(body).await;
    StatusCode::OK
}

pub async fn get_listener(
    State(server): State<Arc<Server>>,
    Path(id): Path<String>,
) -> (StatusCode, Json<Option<ListenerResponse>>) {
    let listener = server.get_listener(&id).await;
    (StatusCode::OK, Json(listener))
}

pub async fn update_listener(
    State(server): State<Arc<Server>>,
    Json(body): Json<ListenerConfig>,
) -> StatusCode {
    server.update_listener(body).await;
    StatusCode::OK
}

pub async fn remove_listener(
    State(server): State<Arc<Server>>,
    Path(id): Path<String>,
) -> StatusCode {
    server.remove_listener(&id).await;
    StatusCode::OK
}
