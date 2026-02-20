use crate::devices::{BaseLoad, Battery, Device, DeviceContext, EvCharger, SolarPv};
use crate::forecast::NaiveForecast;
use crate::scenario::ScenarioConfig;
use crate::sim::clock::Clock;
use crate::sim::controller::NaiveRtController;
use crate::sim::event::DemandResponseEvent;
use crate::sim::feeder::Feeder;
use crate::sim::schedule::DayAheadSchedule;
use crate::telemetry::TelemetryRow;

pub struct SimulationKpis {
    pub rmse_tracking_kw: f32,
    pub curtailment_pct: f32,
    pub feeder_peak_load_kw: f32,
}

pub struct SimulationResult {
    pub telemetry: Vec<TelemetryRow>,
    pub kpis: SimulationKpis,
}

pub fn run_scenario(config: &ScenarioConfig, print_readable_log: bool) -> SimulationResult {
    let houses = config.houses as f32;
    let steps_per_day = config.steps_per_day;
    let dt_hr = 24.0 / steps_per_day as f32;
    let mut clock = Clock::new(steps_per_day); // Simulate 1 day

    let mut load = BaseLoad::new(
        0.8 * houses,  /* base_kw */
        0.7 * houses,  /* amp_kw */
        1.2,           /* phase_rad */
        0.05,          /* noise_std */
        steps_per_day, /* steps_per_day */
        config.seed,   /* seed */
    );

    let baseload_device = load.device_type();
    let mut baseline_load = load.clone();
    let mut baseline = Vec::with_capacity(steps_per_day);
    for t in 0..steps_per_day {
        baseline.push(baseline_load.power_kw(&DeviceContext::new(t)));
    }
    let forecaster = NaiveForecast;
    let load_forecast = forecaster.forecast(&baseline, steps_per_day);
    let target_schedule = DayAheadSchedule::flat_target(&load_forecast);

    let mut pv = SolarPv::new(
        5.0 * houses,                /* kw_peak */
        steps_per_day,               /* steps_per_day */
        6,                           /* sunrise_idx (6 AM) */
        18,                          /* sunset_idx (6 PM) */
        0.05,                        /* noise_std */
        config.seed.wrapping_add(1), /* seed */
    );

    let solar_device = pv.device_type();

    let mut battery = Battery::new(
        10.0 * houses, /* capacity_kwh */
        0.5,           /* initial_soc */
        5.0 * houses,  /* max_charge_kw */
        5.0 * houses,  /* max_discharge_kw */
        0.95,          /* eta_c */
        0.95,          /* eta_d */
        steps_per_day, /* steps_per_day */
    );

    let battery_device = battery.device_type();
    let mut ev = EvCharger::new(
        7.2 * houses,                /* max_charge_kw */
        steps_per_day,               /* steps_per_day */
        4.0 * houses,                /* demand_kwh_min */
        14.0 * houses,               /* demand_kwh_max */
        3,                           /* dwell_steps_min */
        10,                          /* dwell_steps_max */
        config.seed.wrapping_add(2), /* seed */
    );
    let ev_device = ev.device_type();

    let mut feeder = Feeder::with_limits(
        "MainFeeder",
        config.feeder_kw,       /* max_import_kw */
        config.feeder_kw * 0.8, /* max_export_kw */
    );

    // Example external DR event: request 1.5kW reduction from hour 17 to 21.
    let dr_event = DemandResponseEvent::new(17, 21, 1.5 * houses);

    let controller = NaiveRtController;

    let mut telemetry = Vec::with_capacity(steps_per_day);
    let mut tracking_error_sq_sum = 0.0_f32;
    let mut tracking_error_count = 0_usize;
    let mut requested_curtailment_sum_kw = 0.0_f32;
    let mut achieved_curtailment_sum_kw = 0.0_f32;
    let mut feeder_peak_load_kw = 0.0_f32;

    clock.run(|t| {
        let context = DeviceContext::new(t);

        let base_demand_kw_raw = load.power_kw(&context);
        let forecast_kw = load_forecast[context.timestep];
        let target_kw = target_schedule[context.timestep];
        let solar_kw = pv.power_kw(&context);
        let ev_requested_kw = ev.requested_power_kw(&context);

        let dr_requested_kw = dr_event.requested_reduction_at_kw(t);
        let (base_demand_kw, ev_after_dr_kw, dr_achieved_kw) = controller.apply_demand_response_kw(
            base_demand_kw_raw,
            ev_requested_kw,
            dr_requested_kw,
        );

        let net_fixed_kw = base_demand_kw - solar_kw;
        let ev_capped_kw = controller.capped_flexible_load_kw(
            net_fixed_kw,
            ev_after_dr_kw,
            feeder.max_import_kw(),
            battery.max_discharge_kw,
        );
        let ev_context = DeviceContext::with_setpoint(context.timestep, ev_capped_kw);
        let ev_kw = ev.power_kw(&ev_context);

        let net_without_battery = net_fixed_kw + ev_kw;
        let battery_setpoint_kw = controller.constrained_battery_setpoint_kw(
            net_without_battery,
            target_kw,
            feeder.max_import_kw(),
            feeder.max_export_kw(),
            battery.max_charge_kw,
            battery.max_discharge_kw,
        );
        let battery_context = DeviceContext::with_setpoint(context.timestep, battery_setpoint_kw);

        let battery_kw = battery.power_kw(&battery_context);
        feeder.reset();
        feeder.add_net_kw(base_demand_kw);
        feeder.add_net_kw(ev_kw);
        feeder.add_net_kw(-solar_kw);
        feeder.add_net_kw(-battery_kw);
        let feeder_kw = feeder.net_kw();
        let tracking_error_kw = feeder_kw - target_kw;
        let feeder_name = feeder.name();

        tracking_error_sq_sum += tracking_error_kw * tracking_error_kw;
        tracking_error_count += 1;
        requested_curtailment_sum_kw += dr_requested_kw;
        achieved_curtailment_sum_kw += dr_achieved_kw;
        feeder_peak_load_kw = feeder_peak_load_kw.max(feeder_kw);

        let row = TelemetryRow {
            timestep: t,
            time_hr: t as f32 * dt_hr,
            target_kw,
            feeder_kw,
            tracking_error_kw,
            baseload_kw: base_demand_kw,
            solar_kw,
            ev_requested_kw,
            ev_dispatched_kw: ev_kw,
            battery_kw,
            battery_soc: battery.soc,
            dr_requested_kw,
            dr_achieved_kw,
            limit_ok: feeder.within_limits(),
        };
        telemetry.push(row);

        let soc = battery.soc * 100.0;
        if print_readable_log {
            println!(
                "Time (Hr) {t}: {baseload_device}={base_demand_kw:.2} kW, \
                RawBase={base_demand_kw_raw:.2} kW, \
                Forecast={forecast_kw:.2} kW, \
                Target={target_kw:.2} kW, \
                {solar_device}={solar_kw:.2} kW, \
                {ev_device}={ev_kw:.2} kW (Req={ev_requested_kw:.2}, DR={ev_after_dr_kw:.2}, Cap={ev_capped_kw:.2}), \
                {battery_device}={battery_kw:.2} kW (SoC={soc:.1}%), \
                {feeder_name}={feeder_kw:.2} kW, \
                Error={tracking_error_kw:.2} kW, \
                DR(req={dr_requested_kw:.2}, done={dr_achieved_kw:.2}), \
                LimitOK={}",
                feeder.within_limits()
            );
        }
    });

    let rmse_tracking_kw = if tracking_error_count > 0 {
        (tracking_error_sq_sum / tracking_error_count as f32).sqrt()
    } else {
        0.0
    };

    let curtailment_pct = if requested_curtailment_sum_kw > 0.0 {
        100.0 * achieved_curtailment_sum_kw / requested_curtailment_sum_kw
    } else {
        0.0
    };

    SimulationResult {
        telemetry,
        kpis: SimulationKpis {
            rmse_tracking_kw,
            curtailment_pct,
            feeder_peak_load_kw,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::run_scenario;
    use crate::scenario::ScenarioConfig;
    use crate::telemetry::write_telemetry_csv;

    #[test]
    fn same_scenario_and_seed_is_deterministic() {
        let scenario = ScenarioConfig {
            houses: 3,
            feeder_kw: 40.0,
            seed: 777,
            steps_per_day: 24,
        };

        let run_a = run_scenario(&scenario, false);
        let run_b = run_scenario(&scenario, false);

        let mut out_a = Vec::new();
        write_telemetry_csv(&mut out_a, &run_a.telemetry).expect("first export should succeed");

        let mut out_b = Vec::new();
        write_telemetry_csv(&mut out_b, &run_b.telemetry).expect("second export should succeed");

        assert_eq!(out_a, out_b);
    }
}
