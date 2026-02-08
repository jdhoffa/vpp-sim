use crate::devices::types::{Device, DeviceContext, gaussian_noise};
use crate::sim::types::SimConfig;
use rand::{SeedableRng, rngs::StdRng};

/// A solar PV generator that models power generation based on daylight hours.
///
/// `SolarPv` creates a half-cosine shaped generation profile between sunrise and sunset
/// times with configurable peak power output and random noise to simulate
/// variations due to weather conditions.
///
/// # Power Flow Convention (Feeder)
/// Returns **negative** values during daylight (generation reduces feeder load).
#[derive(Debug, Clone)]
pub struct SolarPv {
    /// Maximum power output in kilowatts under ideal conditions.
    pub kw_peak: f32,

    /// Number of time steps per simulated day.
    steps_per_day: usize,

    /// Time step index when sunrise occurs (inclusive).
    pub sunrise_idx: usize,

    /// Time step index when sunset occurs (exclusive).
    pub sunset_idx: usize,

    /// Standard deviation of the Gaussian noise as a fraction of output.
    pub noise_std: f32,

    /// Random number generator for noise generation.
    rng: StdRng,
}

impl SolarPv {
    /// Creates a new solar PV generator with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `kw_peak` - Maximum power output in kilowatts under ideal conditions
    /// * `sunrise_idx` - Time step index when sunrise occurs (inclusive)
    /// * `sunset_idx` - Time step index when sunset occurs (exclusive)
    /// * `noise_std` - Standard deviation of noise (e.g., 0.05 for +/-5% variation)
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
        noise_std: f32,
        config: &SimConfig,
        seed: u64,
    ) -> Self {
        assert!(sunrise_idx < sunset_idx && sunset_idx <= config.steps_per_day);
        Self {
            kw_peak: kw_peak.max(0.0),
            steps_per_day: config.steps_per_day,
            sunrise_idx,
            sunset_idx,
            noise_std: noise_std.max(0.0),
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Calculates the daylight fraction for a specific time step.
    ///
    /// Delegates to the shared [`super::types::daylight_frac`] free function.
    fn daylight_frac(&self, t: usize) -> f32 {
        super::types::daylight_frac(t, self.steps_per_day, self.sunrise_idx, self.sunset_idx)
    }
}

impl Device for SolarPv {
    /// Calculates the power generation at a specific time step in feeder convention.
    ///
    /// Returns **negative** values during daylight (generation exports to grid).
    /// Returns 0.0 during nighttime hours.
    fn power_kw(&mut self, context: &DeviceContext) -> f32 {
        let frac = self.daylight_frac(context.timestep);
        if frac <= 0.0 {
            return 0.0;
        }

        let noise_mult = 1.0 + gaussian_noise(&mut self.rng, self.noise_std);
        let kw = self.kw_peak * frac * noise_mult;

        // Return negative for generation (feeder convention: negative = export)
        -(kw.max(0.0))
    }

    fn device_type(&self) -> &'static str {
        "SolarPV"
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
    fn test_new_solar_pv() {
        let c = cfg();
        let pv = SolarPv::new(5.0, 6, 18, 0.05, &c, 42);
        assert_eq!(pv.kw_peak, 5.0);
        assert_eq!(pv.steps_per_day, 24);
        assert_eq!(pv.sunrise_idx, 6);
        assert_eq!(pv.sunset_idx, 18);
        assert_eq!(pv.noise_std, 0.05);
    }

    #[test]
    fn test_negative_kw_peak_clamped_to_zero() {
        let pv = SolarPv::new(-1.0, 6, 18, 0.05, &cfg(), 42);
        assert_eq!(pv.kw_peak, 0.0);
    }

    #[test]
    fn test_negative_noise_std_clamped_to_zero() {
        let pv = SolarPv::new(5.0, 6, 18, -0.05, &cfg(), 42);
        assert_eq!(pv.noise_std, 0.0);
    }

    #[test]
    #[should_panic]
    fn test_sunset_before_sunrise_panics() {
        SolarPv::new(5.0, 18, 6, 0.05, &cfg(), 42);
    }

    #[test]
    #[should_panic]
    fn test_sunset_exceeds_steps_panics() {
        SolarPv::new(5.0, 6, 25, 0.05, &cfg(), 42);
    }

    #[test]
    fn test_daylight_frac() {
        let c = cfg();
        let pv = SolarPv::new(5.0, 6, 18, 0.0, &c, 42);

        assert_eq!(pv.daylight_frac(0), 0.0);
        assert_eq!(pv.daylight_frac(5), 0.0);
        assert_eq!(pv.daylight_frac(18), 0.0);
        assert_eq!(pv.daylight_frac(23), 0.0);

        let dawn_frac = pv.daylight_frac(6);
        assert!(dawn_frac < 0.1);

        let noon_frac = pv.daylight_frac(12);
        assert!(noon_frac > 0.95);

        assert!((pv.daylight_frac(9) - pv.daylight_frac(15)).abs() < 1e-5);
    }

    #[test]
    fn test_no_generation_at_night() {
        let mut pv = SolarPv::new(5.0, 6, 18, 0.0, &cfg(), 42);
        assert_eq!(pv.power_kw(&ctx(0)), 0.0);
        assert_eq!(pv.power_kw(&ctx(5)), 0.0);
        assert_eq!(pv.power_kw(&ctx(18)), 0.0);
        assert_eq!(pv.power_kw(&ctx(23)), 0.0);
    }

    #[test]
    fn test_peak_generation_at_noon() {
        let mut pv = SolarPv::new(5.0, 6, 18, 0.0, &cfg(), 42);
        let noon_gen = pv.power_kw(&ctx(12));
        // Feeder convention: generation is negative
        assert!(noon_gen < -4.9 && noon_gen >= -5.0);
    }

    #[test]
    fn test_deterministic_with_same_seed() {
        let c = cfg();
        let mut pv1 = SolarPv::new(5.0, 6, 18, 0.1, &c, 42);
        let mut pv2 = SolarPv::new(5.0, 6, 18, 0.1, &c, 42);

        for t in 0..24 {
            assert_eq!(pv1.power_kw(&ctx(t)), pv2.power_kw(&ctx(t)));
        }
    }

    #[test]
    fn test_different_seeds_produce_different_results() {
        let c = cfg();
        let mut pv1 = SolarPv::new(5.0, 6, 18, 0.1, &c, 42);
        let mut pv2 = SolarPv::new(5.0, 6, 18, 0.1, &c, 43);

        let mut all_same = true;
        for t in 6..18 {
            if (pv1.power_kw(&ctx(t)) - pv2.power_kw(&ctx(t))).abs() > 1e-5 {
                all_same = false;
                break;
            }
        }
        assert!(!all_same);
    }

    #[test]
    fn test_solar_always_negative_or_zero() {
        let mut pv = SolarPv::new(5.0, 6, 18, 0.05, &cfg(), 42);
        for t in 0..48 {
            assert!(pv.power_kw(&ctx(t)) <= 0.0);
        }
    }
}
