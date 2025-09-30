mod devices;
mod sim;

use devices::{BaseLoad, Battery, Device, DeviceContext, SolarPv};
use sim::clock::Clock;

fn main() {
    let steps_per_day = 24; // 1-hr intervals
    let mut clock = Clock::new(steps_per_day); // Simulate 1 days

    let mut load = BaseLoad::new(
        0.8,           /* base_kw */
        0.7,           /* amp_kw */
        1.2,           /* phase_rad */
        0.05,          /* noise_std */
        steps_per_day, /* steps_per_day */
        42,            /* seed */
    );

    let baseload_device = load.device_type();

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

    clock.run(|t| {
        let context = DeviceContext::new(t);

        let base_demand_kw = load.power_kw(&context);
        let solar_kw = pv.power_kw(&context);

        // Simple battery control strategy:
        // - If solar excess (negative net load), charge battery with excess
        // - If net load positive, discharge battery to meet load, up to max discharge
        let net_without_battery = base_demand_kw - solar_kw;

        let battery_context = DeviceContext::with_setpoint(context.timestep, net_without_battery);

        let battery_kw = battery.power_kw(&battery_context);
        let net_with_battery = net_without_battery - battery_kw;

        let soc = battery.soc * 100.0;
        println!(
            "Time (Hr) {t}: {baseload_device}={base_demand_kw:.2} kW, \
            {solar_device}={solar_kw:.2} kW, \
            {battery_device}={battery_kw:.2} kW (SoC={soc:.1}%), \
            Net={net_with_battery:.2} kW"
        );
        // later: push `kw` into feeder aggregator
    })
}
