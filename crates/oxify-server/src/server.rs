//! HTTP server runtime with Axum and graceful shutdown

use crate::{
    shutdown::shutdown_signal,
    types::{Result, ServerConfig, ServerError},
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde_json::json;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing::info;

/// HTTP server runtime
pub struct ServerRuntime {
    config: ServerConfig,
    router: Option<Router>,
}

impl ServerRuntime {
    /// Create a new server runtime
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            router: None,
        }
    }

    /// Create with development configuration
    pub fn development() -> Self {
        Self::new(ServerConfig::development())
    }

    /// Create with production configuration
    pub fn production(config: ServerConfig) -> Self {
        Self::new(config)
    }

    /// Set custom router
    pub fn with_router(mut self, router: Router) -> Self {
        self.router = Some(router);
        self
    }

    /// Build the application router with middleware
    fn build_app(&self) -> Router {
        let app = self.router.clone().unwrap_or_else(|| self.default_router());

        // Middleware stack
        let app = app
            .layer(TraceLayer::new_for_http());

        // Add compression if enabled
        #[cfg(feature = "compression")]
        let app = if self.config.compression {
            app.layer(CompressionLayer::new())
        } else {
            app
        };

        #[cfg(not(feature = "compression"))]
        let app = app;

        // Add request logging
        let app = if self.config.request_logging {
            app.layer(tower_http::trace::TraceLayer::new_for_http())
        } else {
            app
        };

        app
    }

    /// Default router with health check endpoints
    fn default_router(&self) -> Router {
        Router::new()
            .route("/health", get(health_check))
            .route("/ready", get(ready_check))
    }

    /// Run the server
    pub async fn run(&self) -> Result<()> {
        let app = self.build_app();
        let addr = self.config.address;

        info!("Server starting on {}", addr);

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| ServerError::BindError(e.to_string()))?;

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal(Duration::from_secs(self.config.graceful_shutdown_timeout)))
            .await
            .map_err(|e| ServerError::ServeError(e.to_string()))?;

        Ok(())
    }
}

/// Health check handler
async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "dag-engine"
    }))
}

/// Readiness check handler
async fn ready_check() -> impl IntoResponse {
    Json(json!({
        "status": "ready"
    }))
}
