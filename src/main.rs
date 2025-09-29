mod devices;
mod sim;

use devices::{BaseLoad, Battery, Device, DeviceContext, SolarPv};
use sim::clock::Clock;

fn main() {
    let steps_per_day = 96; // 15-minute intervals
    let mut clock = Clock::new(steps_per_day * 2); // Simulate 2 days

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
        24,            /* sunrise_idx (6 AM) */
        72,            /* sunset_idx (6 PM) */
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
        let base_context = DeviceContext {
            timestep: t,
            setpoint_kw: None,
        };

        let solar_context = DeviceContext {
            timestep: t,
            setpoint_kw: None,
        };

        let base_demand_kw = load.power_kw(&base_context);
        let solar_kw = pv.power_kw(&solar_context);

        // Simple battery control strategy:
        // - If solar excess (negative net load), charge battery with excess
        // - If net load positive, discharge battery to meet load, up to max discharge
        let net_without_battery = base_demand_kw + solar_kw;

        let battery_context = DeviceContext {
            timestep: t,
            setpoint_kw: Some(-net_without_battery), // Negative to charge, positive to discharge
        };

        let battery_kw = battery.power_kw(&battery_context);
        let net_with_battery = net_without_battery + battery_kw;

        println!(
            "Timestep {}: {}={:.2}kW, {}={:.2}kW, {}={:.2}kW (SoC={:.1}%), Net={:.2}kW",
            t,
            baseload_device,
            base_demand_kw,
            solar_device,
            solar_kw,
            battery_device,
            battery_kw,
            battery.soc * 100.0,
            net_with_battery
        );
        // later: push `kw` into feeder aggregator
    })
}
