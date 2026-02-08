//! Shared test fixtures for integration tests.

use vpp_sim::devices::{BaseLoad, Battery, Device, DeviceContext, EvCharger, SolarPv};
use vpp_sim::forecast::NaiveForecast;
use vpp_sim::sim::event::DemandResponseEvent;
use vpp_sim::sim::feeder::Feeder;
use vpp_sim::sim::schedule::DayAheadSchedule;
use vpp_sim::sim::types::SimConfig;

/// Default simulation configuration (24 steps/day, 1 day, seed 42).
pub fn default_config() -> SimConfig {
    SimConfig::new(24, 1, 42)
}

/// Default baseline forecast vector and baseload device.
///
/// Returns `(baseline, load)` where `baseline` is the per-step forecast
/// input and `load` is a fresh `BaseLoad` with the same parameters.
pub fn default_baseline_and_load(config: &SimConfig) -> (Vec<f32>, BaseLoad) {
    let load = BaseLoad::new(0.8, 0.7, 1.2, 0.05, config, 42);
    let mut baseline_load = load.clone();
    let mut baseline = Vec::with_capacity(config.steps_per_day);
    for t in 0..config.steps_per_day {
        baseline.push(baseline_load.power_kw(&DeviceContext::new(t)));
    }
    (baseline, load)
}

/// Default solar PV device (5 kW peak, sunrise 6, sunset 18).
pub fn default_solar_pv(config: &SimConfig) -> SolarPv {
    SolarPv::new(5.0, 6, 18, 0.05, config, 42)
}

/// Default battery (10 kWh, 50% SOC, 5 kW charge/discharge, 95% efficiency).
pub fn default_battery(config: &SimConfig) -> Battery {
    Battery::new(10.0, 0.5, 5.0, 5.0, 0.95, 0.95, config)
}

/// Default EV charger (7.2 kW, 4–14 kWh demand, 3–10 step dwell).
pub fn default_ev(config: &SimConfig) -> EvCharger {
    EvCharger::new(7.2, 4.0, 14.0, 3, 10, config, 99)
}

/// Default feeder (5 kW import, 4 kW export).
pub fn default_feeder() -> Feeder {
    Feeder::with_limits("MainFeeder", 5.0, 4.0)
}

/// Default demand response event (steps 17–21, 1.5 kW reduction).
pub fn default_dr_event() -> DemandResponseEvent {
    DemandResponseEvent::new(17, 21, 1.5)
}

/// Default load forecast and day-ahead schedule derived from baseline.
///
/// Returns `(load_forecast, target_schedule)`.
pub fn default_forecast_and_schedule(
    baseline: &[f32],
    steps_per_day: usize,
) -> (Vec<f32>, Vec<f32>) {
    let forecaster = NaiveForecast;
    let load_forecast = forecaster.forecast(baseline, steps_per_day);
    let target_schedule = DayAheadSchedule::flat_target(&load_forecast);
    (load_forecast, target_schedule)
}
