//! Request handlers for the API endpoints.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use super::AppState;
use super::types::{ErrorResponse, StateResponse, TelemetryQuery, TelemetryRecord};

/// Returns simulation config, KPI report, and latest telemetry record.
///
/// `GET /state` → 200 + `StateResponse` JSON
pub async fn get_state(State(state): State<Arc<AppState>>) -> Json<StateResponse> {
    let latest = state.results.last().map_or_else(
        || TelemetryRecord::from(&default_step()),
        TelemetryRecord::from,
    );

    Json(StateResponse {
        config: state.config.clone(),
        kpi: state.kpi.clone(),
        latest_step: latest,
    })
}

/// Returns telemetry records, optionally filtered by timestep range.
///
/// `GET /telemetry` → 200 + `Vec<TelemetryRecord>` JSON
/// `GET /telemetry?from=N&to=M` → filtered range (inclusive)
/// `GET /telemetry?from=10&to=5` → 400 + `ErrorResponse`
pub async fn get_telemetry(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TelemetryQuery>,
) -> impl IntoResponse {
    let from = query.from.unwrap_or(0);
    let to = query.to.unwrap_or(usize::MAX);

    if from > to {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("`from` ({from}) must be <= `to` ({to})"),
            }),
        ));
    }

    let records: Vec<TelemetryRecord> = state
        .results
        .iter()
        .filter(|r| r.timestep >= from && r.timestep <= to)
        .map(TelemetryRecord::from)
        .collect();

    Ok(Json(records))
}

/// Produces a zeroed `StepResult` for the edge case of an empty results vec.
fn default_step() -> crate::sim::types::StepResult {
    crate::sim::types::StepResult {
        timestep: 0,
        time_hr: 0.0,
        base_kw_raw: 0.0,
        base_kw_after_dr: 0.0,
        solar_kw: 0.0,
        ev_requested_kw: 0.0,
        ev_after_dr_kw: 0.0,
        ev_cap_kw: 0.0,
        ev_actual_kw: 0.0,
        battery_setpoint_kw: 0.0,
        battery_actual_kw: 0.0,
        battery_soc: 0.0,
        feeder_kw: 0.0,
        target_kw: 0.0,
        tracking_error_kw: 0.0,
        dr_requested_kw: 0.0,
        dr_achieved_kw: 0.0,
        within_feeder_limits: true,
        imbalance_cost: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use tower::util::ServiceExt;

    use super::*;
    use crate::api::router;
    use crate::sim::kpi::KpiReport;
    use crate::sim::types::SimConfig;

    fn make_test_state() -> Arc<AppState> {
        let config = SimConfig::new(24, 1, 42);
        let results: Vec<crate::sim::types::StepResult> = (0..24)
            .map(|t| crate::sim::types::StepResult {
                timestep: t,
                time_hr: t as f32,
                base_kw_raw: 1.0,
                base_kw_after_dr: 0.9,
                solar_kw: -2.5,
                ev_requested_kw: 3.0,
                ev_after_dr_kw: 2.5,
                ev_cap_kw: 2.5,
                ev_actual_kw: 2.5,
                battery_setpoint_kw: -1.0,
                battery_actual_kw: -1.0,
                battery_soc: 0.48,
                feeder_kw: -0.1,
                target_kw: 0.0,
                tracking_error_kw: -0.1,
                dr_requested_kw: 0.0,
                dr_achieved_kw: 0.0,
                within_feeder_limits: true,
                imbalance_cost: 0.01,
            })
            .collect();
        let kpi = KpiReport::from_results(&results, config.dt_hours, 10.0);
        Arc::new(AppState {
            config,
            kpi,
            results,
        })
    }

    #[tokio::test]
    async fn state_returns_200() {
        let state = make_test_state();
        let app = router(state);

        let req = Request::builder()
            .uri("/state")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("config").is_some());
        assert!(json.get("kpi").is_some());
        assert!(json.get("latest_step").is_some());
    }

    #[tokio::test]
    async fn telemetry_returns_all_steps() {
        let state = make_test_state();
        let app = router(state);

        let req = Request::builder()
            .uri("/telemetry")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.len(), 24);
    }

    #[tokio::test]
    async fn telemetry_range_query() {
        let state = make_test_state();
        let app = router(state);

        let req = Request::builder()
            .uri("/telemetry?from=5&to=10")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.len(), 6); // timesteps 5,6,7,8,9,10
        assert_eq!(json[0]["timestep"], 5);
        assert_eq!(json[5]["timestep"], 10);
    }

    #[tokio::test]
    async fn telemetry_invalid_range_returns_400() {
        let state = make_test_state();
        let app = router(state);

        let req = Request::builder()
            .uri("/telemetry?from=10&to=5")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("error").is_some());
    }
}
