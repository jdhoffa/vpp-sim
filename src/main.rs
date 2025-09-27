mod devices {
    pub mod baseload;
}

mod sim {
    pub mod clock;
}

fn main() {
    let mut clock = sim::clock::Clock::new(20);
    let mut load = devices::baseload::BaseLoad::new(
        0.8,  /* base_kw */
        0.7,  /* amp_kw */
        1.2,  /* phase_rad */
        0.05, /* noise_std */
        96,   /* steps_per_day */
        42,   /* seed */
    );

    clock.run(|t| {
        let kw = load.demand_kw(t);
        println!("t={t}, baseload_kw={kw:.2}");
        // later: push `kw` into feeder aggregator
    })
}
