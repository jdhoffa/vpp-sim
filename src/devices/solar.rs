use rand::{Rng, SeedableRng, rngs::StdRng};

#[derive(Debug, Clone)]
pub struct SolarPv {
    pub kw_peak: f32,
    pub steps_per_day: usize,
    pub sunrise_idx: usize, // inclusive
    pub sunset_idx: usize,  // exclusive
    pub noise_std: f32,     // e.g. 0.05 for +/-5% (Gaussian-ish)
    rng: StdRng,
}

impl SolarPv {
    pub fn new(
        kw_peak: f32,
        steps_per_day: usize,
        sunrise_idx: usize,
        sunset_idx: usize,
        noise_std: f32,
        seed: u64,
    ) -> Self {
        assert!(steps_per_day > 0);
        assert!(sunrise_idx < sunset_idx && sunset_idx <= steps_per_day);
        Self {
            kw_peak: kw_peak.max(0.0),
            steps_per_day,
            sunrise_idx,
            sunset_idx,
            noise_std: noise_std.max(0.0),
            rng: StdRng::seed_from_u64(seed),
        }
    }

    fn daylight_frac(&self, t: usize) -> f32 {
        let day_t = t % self.steps_per_day;
        if day_t < self.sunrise_idx || day_t >= self.sunset_idx {
            return 0.0;
        }
        let span = (self.sunset_idx - self.sunrise_idx) as f32;
        let x = (day_t - self.sunrise_idx) as f32 / span; // [0,1)
        // Half-cosine dome: 0 -> 1 -> 0 across daylight
        0.5 * (1.0 - (std::f32::consts::PI * x).cos())
    }

    pub fn gen_kw(&mut self, timestep: usize) -> f32 {
        let frac = self.daylight_frac(timestep);
        if frac <= 0.0 {
            return 0.0;
        }

        // Gaussian-ish noise via Box–Muller
        let (mut u1, u2): (f32, f32) = (Rng::random(&mut self.rng), Rng::random(&mut self.rng));
        u1 = u1.clamp(1e-6, 1.0);
        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos(); // ~N(0,1)
        let mult = 1.0 + z0 * self.noise_std;

        let kw = self.kw_peak * frac * mult;
        kw.max(0.0)
    }
}
