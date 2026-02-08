//! Solar PV model with temporally correlated cloud variability (AR(1) process).

use crate::devices::types::{Device, DeviceContext, gaussian_noise};
use crate::sim::types::SimConfig;
use rand::{SeedableRng, rngs::StdRng};

/// Solar PV generator with an AR(1) cloud multiplier for realistic variability.
///
/// Unlike [`SolarPv`](super::SolarPv) which applies independent Gaussian noise
/// per timestep, `SolarPvAr1` models temporally correlated cloud fronts via a
/// first-order autoregressive process on a PV multiplier.
///
/// The multiplier evolves as:
/// ```text
/// m(t) = alpha * m(t-1) + (1 - alpha) * epsilon(t)
/// ```
/// where `epsilon` is Gaussian noise and `alpha` controls temporal correlation.
/// The multiplier is clamped to \[0.2, 1.2\].
///
/// # Power Flow Convention (Feeder)
/// Returns **negative** values during daylight (generation reduces feeder load).
#[derive(Debug, Clone)]
pub struct SolarPvAr1 {
    /// Maximum power output in kilowatts under ideal conditions.
    pub kw_peak: f32,

    /// Number of time steps per simulated day.
    steps_per_day: usize,

    /// Time step index when sunrise occurs (inclusive).
    pub sunrise_idx: usize,

    /// Time step index when sunset occurs (exclusive).
    pub sunset_idx: usize,

    /// AR(1) correlation coefficient (0.0 = uncorrelated, 1.0 = fully persistent).
    pub alpha: f32,

    /// Standard deviation of the AR(1) innovation noise.
    pub cloud_noise_std: f32,

    /// Current cloud multiplier state.
    multiplier: f32,

    /// Random number generator for noise generation.
    rng: StdRng,
}

/// Minimum cloud multiplier (heavy overcast).
const MULTIPLIER_MIN: f32 = 0.2;
/// Maximum cloud multiplier (enhanced irradiance from cloud edges).
const MULTIPLIER_MAX: f32 = 1.2;

impl SolarPvAr1 {
    /// Creates a new solar PV generator with AR(1) cloud variability.
    ///
    /// # Arguments
    ///
    /// * `kw_peak` - Maximum power output in kilowatts under ideal conditions
    /// * `sunrise_idx` - Time step index when sunrise occurs (inclusive)
    /// * `sunset_idx` - Time step index when sunset occurs (exclusive)
    /// * `alpha` - AR(1) correlation coefficient (typical: 0.8–0.95)
    /// * `cloud_noise_std` - Standard deviation of innovation noise (typical: 0.15–0.3)
    /// * `config` - Simulation configuration for timing
    /// * `seed` - Random seed for reproducible noise generation
    ///
    /// # Panics
    ///
    /// Panics if `sunrise_idx >= sunset_idx` or `sunset_idx > steps_per_day`.
    pub fn new(
        kw_peak: f32,
        sunrise_idx: usize,
        sunset_idx: usize,
        alpha: f32,
        cloud_noise_std: f32,
        config: &SimConfig,
        seed: u64,
    ) -> Self {
        assert!(
            sunrise_idx < sunset_idx && sunset_idx <= config.steps_per_day,
            "sunrise_idx must be < sunset_idx and sunset_idx must be <= steps_per_day"
        );
        Self {
            kw_peak: kw_peak.max(0.0),
            steps_per_day: config.steps_per_day,
            sunrise_idx,
            sunset_idx,
            alpha: alpha.clamp(0.0, 1.0),
            cloud_noise_std: cloud_noise_std.max(0.0),
            multiplier: 1.0,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Calculates the daylight fraction for a specific time step.
    ///
    /// Delegates to the shared [`super::types::daylight_frac`] free function.
    fn daylight_frac(&self, t: usize) -> f32 {
        super::types::daylight_frac(t, self.steps_per_day, self.sunrise_idx, self.sunset_idx)
    }

    /// Advances the AR(1) cloud multiplier by one step and returns the new value.
    fn advance_multiplier(&mut self) -> f32 {
        let epsilon = gaussian_noise(&mut self.rng, self.cloud_noise_std);
        self.multiplier = self.alpha * self.multiplier + (1.0 - self.alpha) * epsilon;
        self.multiplier = self.multiplier.clamp(MULTIPLIER_MIN, MULTIPLIER_MAX);
        self.multiplier
    }
}

impl Device for SolarPvAr1 {
    /// Calculates the power generation at a specific time step in feeder convention.
    ///
    /// Returns **negative** values during daylight (generation exports to grid).
    /// Returns 0.0 during nighttime hours. The cloud multiplier evolves every
    /// timestep regardless of daylight, maintaining temporal correlation.
    fn power_kw(&mut self, context: &DeviceContext) -> f32 {
        let m = self.advance_multiplier();
        let frac = self.daylight_frac(context.timestep);
        if frac <= 0.0 {
            return 0.0;
        }

        let kw = self.kw_peak * frac * m;
        // Return negative for generation (feeder convention: negative = export)
        -(kw.max(0.0))
    }

    fn device_type(&self) -> &'static str {
        "SolarPV_ar1"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> SimConfig {
        SimConfig::new(24, 1, 0)
    }

    fn ctx(t: usize) -> DeviceContext {
        DeviceContext::new(t)
    }

    #[test]
    fn seed_determinism() {
        let c = cfg();
        let mut pv1 = SolarPvAr1::new(5.0, 6, 18, 0.9, 0.2, &c, 42);
        let mut pv2 = SolarPvAr1::new(5.0, 6, 18, 0.9, 0.2, &c, 42);

        for t in 0..48 {
            assert_eq!(pv1.power_kw(&ctx(t)), pv2.power_kw(&ctx(t)));
        }
    }

    #[test]
    fn different_seeds_produce_different_sequences() {
        let c = cfg();
        let mut pv1 = SolarPvAr1::new(5.0, 6, 18, 0.9, 0.2, &c, 42);
        let mut pv2 = SolarPvAr1::new(5.0, 6, 18, 0.9, 0.2, &c, 99);

        let mut any_differ = false;
        for t in 6..18 {
            if (pv1.power_kw(&ctx(t)) - pv2.power_kw(&ctx(t))).abs() > 1e-5 {
                any_differ = true;
                break;
            }
        }
        assert!(
            any_differ,
            "different seeds should produce different outputs"
        );
    }

    #[test]
    fn multiplier_stays_within_bounds() {
        let c = SimConfig::new(24, 10, 0); // 10 days for more samples
        let mut pv = SolarPvAr1::new(5.0, 6, 18, 0.8, 0.5, &c, 42);

        // Run many steps with high noise to stress-test clamping
        for t in 0..c.total_steps() {
            let kw = pv.power_kw(&ctx(t));
            // During daylight: power should be between -kw_peak*1.2 and 0
            // At night: exactly 0
            assert!(kw <= 0.0, "generation should be <= 0 at t={t}, got {kw}");
            assert!(
                kw >= -5.0 * MULTIPLIER_MAX,
                "power should not exceed peak * max_multiplier at t={t}, got {kw}"
            );
        }
    }

    #[test]
    fn temporal_correlation_adjacent_steps_more_similar() {
        let c = SimConfig::new(96, 1, 0); // 15-min steps for smoother data
        let mut pv = SolarPvAr1::new(5.0, 24, 72, 0.9, 0.2, &c, 42);

        let mut outputs = Vec::with_capacity(c.total_steps());
        for t in 0..c.total_steps() {
            outputs.push(pv.power_kw(&ctx(t)));
        }

        // Compare adjacent-step differences vs 10-step-apart differences
        // during daylight only (steps 24..72)
        let mut adj_diff_sum = 0.0_f32;
        let mut adj_count = 0_u32;
        let mut far_diff_sum = 0.0_f32;
        let mut far_count = 0_u32;

        for t in 25..72 {
            adj_diff_sum += (outputs[t] - outputs[t - 1]).abs();
            adj_count += 1;
            if t >= 34 {
                far_diff_sum += (outputs[t] - outputs[t - 10]).abs();
                far_count += 1;
            }
        }

        let avg_adj = adj_diff_sum / adj_count as f32;
        let avg_far = far_diff_sum / far_count as f32;
        // Adjacent steps should on average be more similar (smaller difference)
        assert!(
            avg_adj < avg_far,
            "adjacent diff ({avg_adj:.4}) should be smaller than distant diff ({avg_far:.4})"
        );
    }

    #[test]
    fn sign_convention_negative_during_daylight() {
        let c = cfg();
        let mut pv = SolarPvAr1::new(5.0, 6, 18, 0.9, 0.2, &c, 42);

        for t in 0..24 {
            let kw = pv.power_kw(&ctx(t));
            assert!(kw <= 0.0, "power should be <= 0 at t={t}, got {kw}");
            // Strictly negative during mid-day (multiplier >= 0.2, frac > 0)
            if t >= 8 && t <= 16 {
                assert!(
                    kw < 0.0,
                    "power should be strictly negative during daylight at t={t}, got {kw}"
                );
            }
        }
    }

    #[test]
    fn no_generation_at_night() {
        let c = cfg();
        let mut pv = SolarPvAr1::new(5.0, 6, 18, 0.9, 0.2, &c, 42);
        assert_eq!(pv.power_kw(&ctx(0)), 0.0);
        assert_eq!(pv.power_kw(&ctx(5)), 0.0);
        assert_eq!(pv.power_kw(&ctx(18)), 0.0);
        assert_eq!(pv.power_kw(&ctx(23)), 0.0);
    }

    #[test]
    #[should_panic]
    fn sunset_before_sunrise_panics() {
        SolarPvAr1::new(5.0, 18, 6, 0.9, 0.2, &cfg(), 42);
    }

    #[test]
    #[should_panic]
    fn sunset_exceeds_steps_panics() {
        SolarPvAr1::new(5.0, 6, 25, 0.9, 0.2, &cfg(), 42);
    }

    #[test]
    fn alpha_clamped_to_unit_interval() {
        let c = cfg();
        let pv = SolarPvAr1::new(5.0, 6, 18, 1.5, 0.2, &c, 42);
        assert_eq!(pv.alpha, 1.0);
        let pv2 = SolarPvAr1::new(5.0, 6, 18, -0.5, 0.2, &c, 42);
        assert_eq!(pv2.alpha, 0.0);
    }
}
