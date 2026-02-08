//! Integration tests for the greedy heuristic controller.

mod common;

use vpp_sim::devices::Solar;
use vpp_sim::sim::controller::{GreedyController, NaiveRtController};
use vpp_sim::sim::engine::Engine;
use vpp_sim::sim::kpi::KpiReport;
use vpp_sim::sim::types::SimConfig;

/// Shared scenario parameters for A/B comparison tests.
struct ScenarioParams {
    config: SimConfig,
    load_forecast: Vec<f32>,
    target_schedule: Vec<f32>,
}

/// Build scenario params using shared fixtures.
fn build_scenario_params() -> ScenarioParams {
    let config = common::default_config();
    let (baseline, _load) = common::default_baseline_and_load(&config);
    let (load_forecast, target_schedule) =
        common::default_forecast_and_schedule(&baseline, config.steps_per_day);

    ScenarioParams {
        config,
        load_forecast,
        target_schedule,
    }
}

/// Build a naive engine with the standard baseline scenario.
fn build_naive_engine(params: &ScenarioParams) -> Engine<NaiveRtController> {
    let config = params.config.clone();
    let (_baseline, load) = common::default_baseline_and_load(&config);
    let pv = Solar::Simple(common::default_solar_pv(&config));
    let battery = common::default_battery(&config);
    let ev = common::default_ev(&config);
    let feeder = common::default_feeder();
    let dr_event = common::default_dr_event();

    Engine::new(
        config,
        load,
        pv,
        battery,
        ev,
        feeder,
        NaiveRtController,
        params.load_forecast.clone(),
        params.target_schedule.clone(),
        dr_event,
    )
}

/// Build a greedy engine with the same baseline scenario.
fn build_greedy_engine(params: &ScenarioParams) -> Engine<GreedyController> {
    let config = params.config.clone();
    let (_baseline, load) = common::default_baseline_and_load(&config);
    let pv = Solar::Simple(common::default_solar_pv(&config));
    let battery = common::default_battery(&config);
    let ev = common::default_ev(&config);
    let feeder = common::default_feeder();
    let dr_event = common::default_dr_event();

    let controller = GreedyController::new(
        &params.load_forecast,
        &params.target_schedule,
        10.0, // capacity_kwh
        5.0,  // max_charge_kw
        5.0,  // max_discharge_kw
        0.5,  // initial_soc
        0.95, // eta_c
        0.95, // eta_d
        config.dt_hours,
        5.0, // solar_kw_peak
        6,   // sunrise_idx
        18,  // sunset_idx
    );

    Engine::new(
        config,
        load,
        pv,
        battery,
        ev,
        feeder,
        controller,
        params.load_forecast.clone(),
        params.target_schedule.clone(),
        dr_event,
    )
}

#[test]
fn greedy_rmse_improves_over_naive_by_at_least_10_percent() {
    let params = build_scenario_params();

    let mut naive_engine = build_naive_engine(&params);
    let naive_results = naive_engine.run();
    let naive_kpi = KpiReport::from_results(
        &naive_results,
        naive_engine.config().dt_hours,
        naive_engine.battery().capacity_kwh,
    );

    let mut greedy_engine = build_greedy_engine(&params);
    let greedy_results = greedy_engine.run();
    let greedy_kpi = KpiReport::from_results(
        &greedy_results,
        greedy_engine.config().dt_hours,
        greedy_engine.battery().capacity_kwh,
    );

    let improvement_pct = 100.0 * (1.0 - greedy_kpi.rmse_tracking_kw / naive_kpi.rmse_tracking_kw);

    eprintln!(
        "RMSE naive={:.4} kW, greedy={:.4} kW, improvement={:.1}%",
        naive_kpi.rmse_tracking_kw, greedy_kpi.rmse_tracking_kw, improvement_pct
    );

    assert!(
        improvement_pct >= 10.0,
        "greedy should improve RMSE by >=10%, got {improvement_pct:.1}% \
         (naive={:.4}, greedy={:.4})",
        naive_kpi.rmse_tracking_kw,
        greedy_kpi.rmse_tracking_kw,
    );
}

#[test]
fn greedy_produces_correct_step_count() {
    let params = build_scenario_params();
    let mut engine = build_greedy_engine(&params);
    let results = engine.run();
    assert_eq!(results.len(), 24);
}

#[test]
fn greedy_kpi_values_are_finite() {
    let params = build_scenario_params();
    let mut engine = build_greedy_engine(&params);
    let results = engine.run();
    let kpi = KpiReport::from_results(
        &results,
        engine.config().dt_hours,
        engine.battery().capacity_kwh,
    );
    assert!(kpi.rmse_tracking_kw.is_finite());
    assert!(kpi.mae_tracking_kw.is_finite());
    assert!(kpi.total_imbalance_cost.is_finite());
    assert!(kpi.battery_throughput_kwh.is_finite());
}

#[test]
fn greedy_feeder_balance_invariant() {
    let params = build_scenario_params();
    let mut engine = build_greedy_engine(&params);
    let results = engine.run();

    for r in &results {
        let expected_feeder =
            r.base_kw_after_dr + r.ev_actual_kw + r.solar_kw + r.battery_actual_kw;
        assert!(
            (r.feeder_kw - expected_feeder).abs() < 1e-4,
            "feeder balance violated at t={}: feeder_kw={}, sum={}",
            r.timestep,
            r.feeder_kw,
            expected_feeder
        );
    }
}

#[test]
fn greedy_determinism() {
    let params = build_scenario_params();
    let mut engine1 = build_greedy_engine(&params);
    let mut engine2 = build_greedy_engine(&params);

    let results1 = engine1.run();
    let results2 = engine2.run();

    for (r1, r2) in results1.iter().zip(results2.iter()) {
        assert_eq!(r1.feeder_kw, r2.feeder_kw);
        assert_eq!(r1.battery_actual_kw, r2.battery_actual_kw);
        assert_eq!(r1.battery_soc, r2.battery_soc);
    }
}

#[test]
fn greedy_imbalance_cost_is_finite() {
    let params = build_scenario_params();
    let mut greedy_engine = build_greedy_engine(&params);
    let greedy_results = greedy_engine.run();
    let greedy_kpi = KpiReport::from_results(
        &greedy_results,
        greedy_engine.config().dt_hours,
        greedy_engine.battery().capacity_kwh,
    );

    assert!(
        greedy_kpi.total_imbalance_cost.is_finite() && greedy_kpi.total_imbalance_cost >= 0.0,
        "imbalance cost should be finite and non-negative, got {:.4}",
        greedy_kpi.total_imbalance_cost,
    );
}
