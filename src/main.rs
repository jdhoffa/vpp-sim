//! VPP simulator entry point — wiring only.

use vpp_sim::devices::{BaseLoad, Battery, Device, DeviceContext, EvCharger, SolarPv};
use vpp_sim::forecast::NaiveForecast;
use vpp_sim::sim::controller::NaiveRtController;
use vpp_sim::sim::engine::Engine;
use vpp_sim::sim::event::DemandResponseEvent;
use vpp_sim::sim::feeder::Feeder;
use vpp_sim::sim::kpi::KpiReport;
use vpp_sim::sim::schedule::DayAheadSchedule;
use vpp_sim::sim::types::SimConfig;

fn main() {
    let config = SimConfig::new(24, 1, 42);

    // Build devices
    let mut baseline_load = BaseLoad::new(0.8, 0.7, 1.2, 0.05, &config, 42);
    let mut baseline = Vec::with_capacity(config.steps_per_day);
    for t in 0..config.steps_per_day {
        baseline.push(baseline_load.power_kw(&DeviceContext::new(t)));
    }

    let load = BaseLoad::new(0.8, 0.7, 1.2, 0.05, &config, 42);
    let pv = SolarPv::new(5.0, 6, 18, 0.05, &config, 42);
    let battery = Battery::new(10.0, 0.5, 5.0, 5.0, 0.95, 0.95, &config);
    let ev = EvCharger::new(7.2, 4.0, 14.0, 3, 10, &config, 99);
    let feeder = Feeder::with_limits("MainFeeder", 5.0, 4.0);

    // Forecast and schedule
    let forecaster = NaiveForecast;
    let load_forecast = forecaster.forecast(&baseline, config.steps_per_day);
    let target_schedule = DayAheadSchedule::flat_target(&load_forecast);

    // DR event and controller
    let dr_event = DemandResponseEvent::new(17, 21, 1.5);
    let controller = NaiveRtController;

    // Build and run engine
    let mut engine = Engine::new(
        config,
        load,
        pv,
        battery,
        ev,
        feeder,
        controller,
        load_forecast,
        target_schedule,
        dr_event,
    );

    let results = engine.run();

    // Print per-step results
    for r in &results {
        println!("{r}");
    }

    // Print KPI report
    let kpi = KpiReport::from_results(
        &results,
        engine.config().dt_hours,
        engine.battery().capacity_kwh,
    );
    println!("\n{kpi}");
}
