mod devices {
    pub mod baseload;
    pub mod solar;
}

mod sim {
    pub mod clock;
}

fn main() {
    let steps_per_day = 96; // 15-minute intervals
    let mut clock = sim::clock::Clock::new(steps_per_day * 2); // Simulate 2 days

    let mut load = devices::baseload::BaseLoad::new(
        0.8,           /* base_kw */
        0.7,           /* amp_kw */
        1.2,           /* phase_rad */
        0.05,          /* noise_std */
        steps_per_day, /* steps_per_day */
        42,            /* seed */
    );

    let mut pv = devices::solar::SolarPv::new(
        5.0,           /* kw_peak */
        steps_per_day, /* steps_per_day */
        24,            /* sunrise_idx (6 AM) */
        72,            /* sunset_idx (6 PM) */
        0.05,          /* noise_std */
        42,            /* seed */
    );

    clock.run(|t| {
        let base_demand_kw = load.demand_kw(t);
        let solar_kw = pv.gen_kw(t);
        let net_kw = base_demand_kw - solar_kw;
        println!(
            "t={t}, baseload_kw={base_demand_kw:.2}, solar_kw={solar_kw:.2}, net_kw={net_kw:.2}"
        );
        // later: push `kw` into feeder aggregator
    })
}
