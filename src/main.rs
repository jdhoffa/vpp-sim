mod devices;
mod sim;

use devices::{BaseLoad, Device, DeviceContext, SolarPv};
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

        println!(
            "Timestep {}: {}={:.2}kW, {}={:.2}kW",
            t, baseload_device, base_demand_kw, solar_device, solar_kw
        );
        // later: push `kw` into feeder aggregator
    })
}
