mod devices;
mod forecast;
mod sim;

use devices::{BaseLoad, Battery, Device, DeviceContext, EvCharger, SolarPv};
use forecast::NaiveForecast;
use sim::clock::Clock;
use sim::controller::NaiveRtController;
use sim::feeder::Feeder;
use sim::schedule::DayAheadSchedule;

fn main() {
    let steps_per_day = 24; // 1-hr intervals
    let mut clock = Clock::new(steps_per_day); // Simulate 1 day

    let mut load = BaseLoad::new(
        0.8,           /* base_kw */
        0.7,           /* amp_kw */
        1.2,           /* phase_rad */
        0.05,          /* noise_std */
        steps_per_day, /* steps_per_day */
        42,            /* seed */
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
        5.0,           /* kw_peak */
        steps_per_day, /* steps_per_day */
        6,             /* sunrise_idx (6 AM) */
        18,            /* sunset_idx (6 PM) */
        0.05,          /* noise_std */
        42,            /* seed */
    );

    let solar_device = pv.device_type();

    let mut battery = Battery::new(
        10.0,          /* capacity_kwh */
        0.5,           /* initial_soc */
        5.0,           /* max_charge_kw */
        5.0,           /* max_discharge_kw */
        0.95,          /* eta_c */
        0.95,          /* eta_d */
        steps_per_day, /* steps_per_day */
    );

    let battery_device = battery.device_type();
    let mut ev = EvCharger::new(
        7.2,           /* max_charge_kw */
        steps_per_day, /* steps_per_day */
        4.0,           /* demand_kwh_min */
        14.0,          /* demand_kwh_max */
        3,             /* dwell_steps_min */
        10,            /* dwell_steps_max */
        99,            /* seed */
    );
    let ev_device = ev.device_type();

    let mut feeder = Feeder::new("MainFeeder");
    let controller = NaiveRtController;

    clock.run(|t| {
        let context = DeviceContext::new(t);

        let base_demand_kw = load.power_kw(&context);
        let forecast_kw = load_forecast[context.timestep];
        let target_kw = target_schedule[context.timestep];
        let solar_kw = pv.power_kw(&context);
        let ev_kw = ev.power_kw(&context);

        let net_without_battery = base_demand_kw + ev_kw - solar_kw;
        let battery_setpoint_kw = controller.battery_setpoint_kw(net_without_battery, target_kw);
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

        let soc = battery.soc * 100.0;
        println!(
            "Time (Hr) {t}: {baseload_device}={base_demand_kw:.2} kW, \
            Forecast={forecast_kw:.2} kW, \
            Target={target_kw:.2} kW, \
            {solar_device}={solar_kw:.2} kW, \
            {ev_device}={ev_kw:.2} kW, \
            {battery_device}={battery_kw:.2} kW (SoC={soc:.1}%), \
            {feeder_name}={feeder_kw:.2} kW, \
            Error={tracking_error_kw:.2} kW"
        );
    })
}
