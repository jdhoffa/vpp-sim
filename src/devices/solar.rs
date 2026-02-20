use crate::devices::types::{Device, DeviceContext, gaussian_noise};
use rand::{SeedableRng, rngs::StdRng};

/// A solar PV generator that models power generation based on daylight hours.
///
/// `SolarPv` creates a half-cosine shaped generation profile between sunrise and sunset
/// times with configurable peak power output and random noise to simulate
/// variations due to weather conditions.
///
/// # Examples
///
/// ```
/// use vpp_sim::devices::solar::SolarPv;
/// use vpp_sim::devices::types::{Device, DeviceContext};
///
/// // Create a solar PV system (5kW peak, 24 steps per day, sunrise at 6am, sunset at 6pm)
/// let mut pv = SolarPv::new(
///     5.0,   // kw_peak - maximum output in ideal conditions
///     24,    // steps_per_day - hourly resolution
///     6,     // sunrise_idx - 6am sunrise
///     18,    // sunset_idx - 6pm sunset
///     0.05,  // noise_std - small random variation for cloud cover
///     42,    // seed - for reproducible randomness
/// );
///
/// // Get generation at noon (step 12)
/// let generation = pv.power_kw(&DeviceContext::new(12));
/// ```
#[derive(Debug, Clone)]
pub struct SolarPv {
    /// Maximum power output in kilowatts under ideal conditions
    pub kw_peak: f32,

    /// Number of time steps per simulated day
    pub steps_per_day: usize,

    /// Time step index when sunrise occurs (inclusive)
    pub sunrise_idx: usize, // inclusive

    /// Time step index when sunset occurs (exclusive)
    pub sunset_idx: usize, // exclusive

    /// Standard deviation of the Gaussian noise as a fraction of output
    pub noise_std: f32, // e.g. 0.05 for +/-5% (Gaussian-ish)

    /// Random number generator for noise generation
    rng: StdRng,
}

impl SolarPv {
    /// Creates a new solar PV generator with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `kw_peak` - Maximum power output in kilowatts under ideal conditions
    /// * `steps_per_day` - Number of time steps per simulated day
    /// * `sunrise_idx` - Time step index when sunrise occurs (inclusive)
    /// * `sunset_idx` - Time step index when sunset occurs (exclusive)
    /// * `noise_std` - Standard deviation of noise (e.g., 0.05 for ±5% variation)
    /// * `seed` - Random seed for reproducible noise generation
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - `steps_per_day` is zero
    /// - `sunrise_idx` is greater than `sunset_idx`
    /// - `sunset_idx` exceeds `steps_per_day`
    ///
    /// # Returns
    ///
    /// A new `SolarPv` instance configured with the specified parameters
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

    /// Calculates the daylight fraction for a specific time step.
    ///
    /// Returns a value between 0.0 and 1.0 representing the relative
    /// solar intensity, following a half-cosine shape from sunrise to sunset.
    /// Returns 0.0 during nighttime hours.
    ///
    /// # Arguments
    ///
    /// * `t` - The simulation time step
    ///
    /// # Returns
    ///
    /// A fraction between 0.0 and 1.0 representing the relative solar intensity
    fn daylight_frac(&self, t: usize) -> f32 {
        let day_t = t % self.steps_per_day;
        if day_t < self.sunrise_idx || day_t >= self.sunset_idx {
            return 0.0;
        }
        let span = (self.sunset_idx - self.sunrise_idx) as f32;
        let x = (day_t - self.sunrise_idx) as f32 / span; // [0,1)
        // Half-cosine dome: 0 -> 1 -> 0 across daylight
        0.5 * (1.0 - (2.0 * std::f32::consts::PI * x).cos())
    }
}

impl Device for SolarPv {
    /// Calculates the power generation at a specific time step.
    ///
    /// This method computes the power generation as a combination of:
    /// - Base solar output following a half-cosine curve during daylight hours
    /// - Random Gaussian noise to simulate variations due to cloud cover
    ///
    /// The generation is guaranteed to be non-negative.
    ///
    /// # Arguments
    ///
    /// * `timestep` - The simulation time step
    ///
    /// # Returns
    ///
    /// The power generation in kilowatts at the specified time step
    fn power_kw(&mut self, context: &DeviceContext) -> f32 {
        let frac = self.daylight_frac(context.timestep);
        if frac <= 0.0 {
            return 0.0;
        }

        let noise_mult = 1.0 + gaussian_noise(&mut self.rng, self.noise_std);
        let kw = self.kw_peak * frac * noise_mult;

        // Return positive for generation (according to power flow convention)
        kw.max(0.0)
    }

    fn device_type(&self) -> &'static str {
        "SolarPV"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a context with just a timestep
    fn ctx(t: usize) -> DeviceContext {
        DeviceContext::new(t)
    }

    #[test]
    fn test_new_solar_pv() {
        let pv = SolarPv::new(5.0, 24, 6, 18, 0.05, 42);
        assert_eq!(pv.kw_peak, 5.0);
        assert_eq!(pv.steps_per_day, 24);
        assert_eq!(pv.sunrise_idx, 6);
        assert_eq!(pv.sunset_idx, 18);
        assert_eq!(pv.noise_std, 0.05);
    }

    #[test]
    fn test_negative_kw_peak_clamped_to_zero() {
        let pv = SolarPv::new(-1.0, 24, 6, 18, 0.05, 42);
        assert_eq!(pv.kw_peak, 0.0);
    }

    #[test]
    fn test_negative_noise_std_clamped_to_zero() {
        let pv = SolarPv::new(5.0, 24, 6, 18, -0.05, 42);
        assert_eq!(pv.noise_std, 0.0);
    }

    #[test]
    #[should_panic]
    fn test_zero_steps_per_day_panics() {
        SolarPv::new(5.0, 0, 0, 1, 0.05, 42);
    }

    #[test]
    #[should_panic]
    fn test_sunset_before_sunrise_panics() {
        SolarPv::new(5.0, 24, 18, 6, 0.05, 42);
    }

    #[test]
    #[should_panic]
    fn test_sunset_exceeds_steps_panics() {
        SolarPv::new(5.0, 24, 6, 25, 0.05, 42);
    }

    #[test]
    fn test_daylight_frac() {
        let pv = SolarPv::new(5.0, 24, 6, 18, 0.0, 42);

        // Night hours return 0.0
        assert_eq!(pv.daylight_frac(0), 0.0); // Midnight
        assert_eq!(pv.daylight_frac(5), 0.0); // 5am
        assert_eq!(pv.daylight_frac(18), 0.0); // 6pm
        assert_eq!(pv.daylight_frac(23), 0.0); // 11pm

        // Dawn starts with near-zero
        let dawn_frac = dbg!(pv.daylight_frac(6));
        assert!(dawn_frac < 0.1);

        // Noon (12) should be near peak (max value)
        let noon_frac = pv.daylight_frac(12);
        assert!(noon_frac > 0.95);

        // Should be symmetric around noon
        assert!((pv.daylight_frac(9) - pv.daylight_frac(15)).abs() < 1e-5);
    }

    #[test]
    fn test_no_generation_at_night() {
        let mut pv = SolarPv::new(5.0, 24, 6, 18, 0.0, 42);

        // No generation during night hours
        assert_eq!(pv.power_kw(&ctx(0)), 0.0); // Midnight
        assert_eq!(pv.power_kw(&ctx(5)), 0.0); // 5am
        assert_eq!(pv.power_kw(&ctx(18)), 0.0); // 6pm
        assert_eq!(pv.power_kw(&ctx(23)), 0.0); // 11pm
    }

    #[test]
    fn test_peak_generation_at_noon() {
        let mut pv = SolarPv::new(5.0, 24, 6, 18, 0.0, 42);

        // With noise_std = 0, noon should generate close to peak
        let noon_gen = pv.power_kw(&ctx(12)); // power_kw returns positive for generation
        assert!(noon_gen > 4.9 && noon_gen <= 5.0);
    }

    #[test]
    fn test_deterministic_with_same_seed() {
        let mut pv1 = SolarPv::new(5.0, 24, 6, 18, 0.1, 42);
        let mut pv2 = SolarPv::new(5.0, 24, 6, 18, 0.1, 42);

        // Same seed should produce identical generation
        for t in 0..24 {
            assert_eq!(pv1.power_kw(&ctx(t)), pv2.power_kw(&ctx(t)));
        }
    }

    #[test]
    fn test_different_seeds_produce_different_results() {
        let mut pv1 = SolarPv::new(5.0, 24, 6, 18, 0.1, 42);
        let mut pv2 = SolarPv::new(5.0, 24, 6, 18, 0.1, 43);

        // Different seeds should produce different generation
        let mut all_same = true;
        for t in 6..18 {
            // Check only daylight hours
            if (pv1.power_kw(&ctx(t)) - pv2.power_kw(&ctx(t))).abs() > 1e-5 {
                all_same = false;
                break;
            }
        }

        assert!(!all_same);
    }

    #[test]
    fn test_multi_day_cycle() {
        let mut pv = SolarPv::new(5.0, 24, 6, 18, 0.0, 42);

        // Generation should repeat in daily cycles
        for t in 0..24 {
            assert_eq!(pv.power_kw(&ctx(t)), pv.power_kw(&ctx(t + 24)));
        }
    }
}
