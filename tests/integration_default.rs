//! Integration tests for the default simulation scenario.

mod common;

use vpp_sim::devices::{Battery, Device, DeviceContext};
use vpp_sim::sim::controller::NaiveRtController;
use vpp_sim::sim::engine::Engine;
use vpp_sim::sim::feeder::Feeder;
use vpp_sim::sim::kpi::KpiReport;
use vpp_sim::sim::types::SimConfig;

/// Build the default scenario engine used across integration tests.
fn build_default_engine() -> Engine<NaiveRtController> {
    let config = common::default_config();
    let (baseline, load) = common::default_baseline_and_load(&config);
    let pv = common::default_solar_pv(&config);
    let battery = common::default_battery(&config);
    let ev = common::default_ev(&config);
    let feeder = common::default_feeder();
    let (load_forecast, target_schedule) =
        common::default_forecast_and_schedule(&baseline, config.steps_per_day);
    let dr_event = common::default_dr_event();

    Engine::new(
        config,
        load,
        pv,
        battery,
        ev,
        feeder,
        NaiveRtController,
        load_forecast,
        target_schedule,
        dr_event,
    )
}

#[test]
fn full_run_produces_correct_step_count() {
    let mut engine = build_default_engine();
    let results = engine.run();
    assert_eq!(results.len(), 24);
}

#[test]
fn full_run_kpi_values_are_finite() {
    let mut engine = build_default_engine();
    let results = engine.run();
    let kpi = KpiReport::from_results(
        &results,
        engine.config().dt_hours,
        engine.battery().capacity_kwh,
    );
    assert!(kpi.rmse_tracking_kw.is_finite());
    assert!(kpi.mae_tracking_kw.is_finite());
    assert!(kpi.curtailment_pct.is_finite());
    assert!(kpi.peak_import_kw.is_finite());
    assert!(kpi.peak_export_kw.is_finite());
    assert!(kpi.battery_throughput_kwh.is_finite());
    assert!(kpi.battery_equivalent_full_cycles.is_finite());
}

#[test]
fn determinism_two_identical_runs_produce_identical_results() {
    let mut engine1 = build_default_engine();
    let mut engine2 = build_default_engine();

    let results1 = engine1.run();
    let results2 = engine2.run();

    assert_eq!(results1.len(), results2.len());
    for (r1, r2) in results1.iter().zip(results2.iter()) {
        assert_eq!(r1.feeder_kw, r2.feeder_kw);
        assert_eq!(r1.battery_actual_kw, r2.battery_actual_kw);
        assert_eq!(r1.battery_soc, r2.battery_soc);
        assert_eq!(r1.solar_kw, r2.solar_kw);
        assert_eq!(r1.ev_actual_kw, r2.ev_actual_kw);
        assert_eq!(r1.base_kw_after_dr, r2.base_kw_after_dr);
        assert_eq!(r1.tracking_error_kw, r2.tracking_error_kw);
    }
}

#[test]
fn sign_invariant_solar_always_reduces_feeder_during_daylight() {
    let mut engine = build_default_engine();
    let results = engine.run();

    for r in &results {
        // Solar should be <= 0 always (feeder convention: generation is negative)
        assert!(
            r.solar_kw <= 0.0,
            "solar_kw should be <= 0 at t={}, got {}",
            r.timestep,
            r.solar_kw
        );

        // During daylight hours (6-17 inclusive), solar should be strictly negative
        if r.timestep >= 7 && r.timestep <= 17 {
            assert!(
                r.solar_kw < 0.0,
                "solar should generate during daylight at t={}, got {}",
                r.timestep,
                r.solar_kw
            );
        }
    }
}

#[test]
fn energy_conservation_battery_soc_matches_integrated_power() {
    let config = SimConfig::new(24, 1, 42);
    let mut battery = Battery::new(10.0, 0.5, 5.0, 5.0, 1.0, 1.0, &config);

    // Charge at 2kW for 3 hours (feeder convention: positive = charge)
    let initial_soc = battery.soc;
    let mut total_energy_in = 0.0_f32;

    for _ in 0..3 {
        let ctx = DeviceContext::with_setpoint(0, 2.0);
        let actual_kw = battery.power_kw(&ctx);
        total_energy_in += actual_kw * config.dt_hours; // positive energy in
    }

    let expected_soc = initial_soc + total_energy_in / battery.capacity_kwh;
    assert!(
        (battery.soc - expected_soc).abs() < 1e-5,
        "SOC mismatch: got {}, expected {} (energy_in={} kWh)",
        battery.soc,
        expected_soc,
        total_energy_in
    );
}

#[test]
fn energy_conservation_feeder_balance_invariant() {
    let mut engine = build_default_engine();
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
fn feeder_constraints_no_violations_in_relaxed_scenario() {
    let config = common::default_config();
    let (baseline, load) = common::default_baseline_and_load(&config);
    let pv = common::default_solar_pv(&config);
    let battery = common::default_battery(&config);
    let ev = common::default_ev(&config);
    let feeder = Feeder::with_limits("Relaxed", 50.0, 50.0);
    let (load_forecast, target_schedule) =
        common::default_forecast_and_schedule(&baseline, config.steps_per_day);
    let dr_event = common::default_dr_event();

    let mut engine = Engine::new(
        config,
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
    let kpi = KpiReport::from_results(
        &results,
        engine.config().dt_hours,
        engine.battery().capacity_kwh,
    );
    assert_eq!(
        kpi.feeder_violation_count, 0,
        "no violations expected with generous feeder limits"
    );
}
