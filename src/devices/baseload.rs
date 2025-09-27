use rand::{Rng, SeedableRng, rngs::StdRng};

#[derive(Debug, Clone)]
pub struct BaseLoad {
    pub base_kw: f32,
    pub amp_kw: f32,
    pub phase_rad: f32,
    pub noise_std: f32,
    pub steps_per_day: usize,
    rng: StdRng,
}

impl BaseLoad {
    pub fn new(
        base_kw: f32,
        amp_kw: f32,
        phase_rad: f32,
        noise_std: f32,
        steps_per_day: usize,
        seed: u64,
    ) -> Self {
        Self {
            base_kw,
            amp_kw,
            phase_rad,
            noise_std,
            steps_per_day: steps_per_day.max(1),
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Deterministic + noisy kW demand at a timestep
    pub fn demand_kw(&mut self, timestep: usize) -> f32 {
        let day_pos = (timestep % self.steps_per_day) as f32 / self.steps_per_day as f32; // [0,1)
        let angle = 2.0 * std::f32::consts::PI * day_pos + self.phase_rad;
        let sinus = angle.sin();

        let noise = if self.noise_std > 0.0 {
            // simple Gaussian-ish noise via Box-Muller
            let u1: f32 = self.rng.random::<f32>().clamp(1e-6, 1.0);
            let u2: f32 = self.rng.random::<f32>();
            let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos();
            z0 * self.noise_std
        } else {
            0.0
        };

        let kw = self.base_kw + self.amp_kw * sinus + noise;
        kw.max(0.0) // no negative demand
    }
}
