//! Integration tests for the REST API feature.

#![cfg(feature = "api")]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::util::ServiceExt;

use vpp_sim::api::{AppState, router};
use vpp_sim::devices::{BaseLoad, Battery, Device, DeviceContext, EvCharger, Solar, SolarPv};
use vpp_sim::forecast::NaiveForecast;
use vpp_sim::sim::controller::NaiveRtController;
use vpp_sim::sim::engine::Engine;
use vpp_sim::sim::event::DemandResponseEvent;
use vpp_sim::sim::feeder::Feeder;
use vpp_sim::sim::kpi::KpiReport;
use vpp_sim::sim::schedule::DayAheadSchedule;
use vpp_sim::sim::types::SimConfig;

/// Build a full simulation and return the API state.
fn build_api_state() -> Arc<AppState> {
    let config = SimConfig::new(24, 1, 42);

    let mut baseline_load = BaseLoad::new(0.8, 0.7, 1.2, 0.05, &config, 42);
    let mut baseline = Vec::with_capacity(config.steps_per_day);
    for t in 0..config.steps_per_day {
        baseline.push(baseline_load.power_kw(&DeviceContext::new(t)));
    }

    let load = BaseLoad::new(0.8, 0.7, 1.2, 0.05, &config, 42);
    let pv = Solar::Simple(SolarPv::new(5.0, 6, 18, 0.05, &config, 42));
    let battery = Battery::new(10.0, 0.5, 5.0, 5.0, 0.95, 0.95, &config);
    let ev = EvCharger::new(7.2, 4.0, 14.0, 3, 10, &config, 99);
    let feeder = Feeder::with_limits("MainFeeder", 5.0, 4.0);

    let forecaster = NaiveForecast;
    let load_forecast = forecaster.forecast(&baseline, config.steps_per_day);
    let target_schedule = DayAheadSchedule::flat_target(&load_forecast);
    let dr_event = DemandResponseEvent::new(17, 21, 1.5);

    let mut engine = Engine::new(
        config.clone(),
        load,
        pv,
        battery,
        ev,
        feeder,
        NaiveRtController,
        load_forecast,
        target_schedule,
        dr_event,
    );

    let results = engine.run();
    let kpi = KpiReport::from_results(&results, config.dt_hours, 10.0);

    Arc::new(AppState {
        config,
        kpi,
        results,
    })
}

#[tokio::test]
async fn full_scenario_state_endpoint() {
    let state = build_api_state();
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

    // Verify config fields
    assert_eq!(json["config"]["steps_per_day"], 24);
    assert_eq!(json["config"]["days"], 1);
    assert_eq!(json["config"]["seed"], 42);

    // Verify KPI fields are present and finite
    assert!(
        json["kpi"]["rmse_tracking_kw"]
            .as_f64()
            .unwrap()
            .is_finite()
    );
    assert!(json["kpi"]["mae_tracking_kw"].as_f64().unwrap().is_finite());

    // Verify latest_step is the last timestep
    assert_eq!(json["latest_step"]["timestep"], 23);
}

#[tokio::test]
async fn full_scenario_telemetry_endpoint() {
    let state = build_api_state();
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
    let records: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();

    assert_eq!(records.len(), 24);

    // Verify CSV schema v1 field names are used
    let first = &records[0];
    assert!(first.get("baseload_kw").is_some());
    assert!(first.get("ev_dispatched_kw").is_some());
    assert!(first.get("battery_kw").is_some());
    assert!(first.get("limit_ok").is_some());
    assert!(first.get("imbalance_cost").is_some());

    // Verify internal field names are NOT exposed
    assert!(first.get("base_kw_after_dr").is_none());
    assert!(first.get("ev_actual_kw").is_none());
    assert!(first.get("battery_actual_kw").is_none());
    assert!(first.get("within_feeder_limits").is_none());
}

#[tokio::test]
async fn full_scenario_telemetry_range() {
    let state = build_api_state();
    let app = router(state);

    let req = Request::builder()
        .uri("/telemetry?from=10&to=15")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let records: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();

    assert_eq!(records.len(), 6);
    assert_eq!(records[0]["timestep"], 10);
    assert_eq!(records[5]["timestep"], 15);
}
