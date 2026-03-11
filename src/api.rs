use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::config::EnvConfig;
use crate::sources::{SourceConfig, SourceInfo};
use crate::{Server, model::Health};

/// Web API and dashboard for managing [Server] sources.
///
/// ### REST Endpoints
///
/// | Method | Path | Handler |
/// |--------|------|---------|
/// | `GET` | `/sources` | [get_all_sources] |
/// | `POST` | `/sources` | [add_source] |
/// | `GET` | `/sources/{id}` | [get_source] |
/// | `PUT` | `/sources/{id}` | [update_source] |
/// | `DELETE` | `/sources/{id}` | [remove_source] |
/// | `GET` | `/health` | [health] |
pub struct Api {
    env: EnvConfig,
    router: Router,
    server: Arc<Server>,
}

impl Api {
    /// Create a new instance of [Api]
    pub async fn new(server: Arc<Server>) -> anyhow::Result<Self> {
        let env = EnvConfig::from_dotenv()?;
        tracing::info!("starting web api on port {}", env.port);
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let router = Router::new()
            .route("/sources", get(get_all_sources))
            .route("/sources", post(add_source))
            .route("/sources/{id}", get(get_source))
            .route("/sources/{id}", put(update_source))
            .route("/sources/{id}", delete(remove_source))
            .route("/sources/types", get(get_source_types))
            .route("/health", get(health))
            .fallback_service(ServeDir::new("static"))
            .layer(cors)
            .with_state(Arc::clone(&server));
        Ok(Self {
            env,
            router,
            server,
        })
    }

    /// Run [Api]
    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.env.port)).await?;

        axum::serve(listener, self.router.clone())
            .with_graceful_shutdown(self.server.shutdown.clone().cancelled_owned())
            .await?;

        tracing::info!("web api stopped");
        Ok(())
    }
}

pub async fn get_all_sources(
    State(server): State<Arc<Server>>,
) -> (StatusCode, Json<Vec<SourceInfo>>) {
    match server.get_all_sources().await {
        Ok(s) => (StatusCode::OK, Json(s)),
        Err(e) => {
            tracing::error!("failed to get all sources: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(Vec::new()))
        }
    }
}

pub async fn get_source_types(
    State(server): State<Arc<Server>>,
) -> (StatusCode, Json<Vec<serde_json::Value>>) {
    match server.get_source_types().await {
        Ok(s) => (StatusCode::OK, Json(s)),
        Err(e) => {
            tracing::error!("failed to get source types: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(Vec::new()))
        }
    }
}

pub async fn add_source(
    State(server): State<Arc<Server>>,
    Json(body): Json<SourceConfig>,
) -> StatusCode {
    if let Err(e) = server.add_source(&body).await {
        tracing::error!("failed to add source: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::OK
}

pub async fn get_source(
    State(server): State<Arc<Server>>,
    Path(id): Path<String>,
) -> (StatusCode, Json<Option<SourceInfo>>) {
    match server.get_source(&id).await {
        Ok(s) => (StatusCode::OK, Json(s)),
        Err(e) => {
            tracing::error!("failed to get source: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(None))
        }
    }
}

pub async fn update_source(
    State(server): State<Arc<Server>>,
    Json(body): Json<SourceConfig>,
) -> StatusCode {
    if let Err(e) = server.update_source(&body).await {
        tracing::error!("failed to update source: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::OK
}

pub async fn remove_source(
    State(server): State<Arc<Server>>,
    Path(id): Path<String>,
) -> StatusCode {
    if let Err(e) = server.remove_source(&id).await {
        tracing::error!("failed to remove source: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::OK
}

pub async fn health(State(server): State<Arc<Server>>) -> (StatusCode, Json<Health>) {
    match server.health().await {
        Ok(h) => (StatusCode::OK, Json(h)),
        Err(e) => {
            tracing::error!("failed to get health: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Health {
                    ok: false,
                    sources: 0,
                }),
            )
        }
    }
}
