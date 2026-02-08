//! REST API for simulation state and telemetry.
//!
//! Provides two GET endpoints:
//! - `/state` — simulation config, KPI report, and latest step
//! - `/telemetry` — full step results with optional range filtering

mod handlers;
mod types;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::sim::kpi::KpiReport;
use crate::sim::types::{SimConfig, StepResult};

/// Immutable application state shared across all request handlers.
///
/// Constructed once after the simulation run completes and wrapped in
/// `Arc` — no locks needed since all data is read-only.
pub struct AppState {
    /// Simulation configuration used for this run.
    pub config: SimConfig,
    /// Aggregate KPI report.
    pub kpi: KpiReport,
    /// Per-step simulation results.
    pub results: Vec<StepResult>,
}

/// Builds the axum router with all API routes.
///
/// # Arguments
///
/// * `state` - Shared application state
///
/// # Returns
///
/// Configured `Router` ready to serve.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/state", get(handlers::get_state))
        .route("/telemetry", get(handlers::get_telemetry))
        .with_state(state)
}

/// Binds to the given address and serves the API.
///
/// # Arguments
///
/// * `state` - Shared application state
/// * `addr` - Socket address to bind to
///
/// # Panics
///
/// Panics if the TCP listener cannot bind to `addr`.
pub async fn serve(state: Arc<AppState>, addr: SocketAddr) {
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind to {addr}: {e}"));
    eprintln!("API server listening on http://{addr}");
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| panic!("server error: {e}"));
}
