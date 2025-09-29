use crate::devices::types::{Device, DeviceContext, gaussian_noise};
use rand::{SeedableRng, rngs::StdRng};

/// A baseload generator that models daily electricity consumption patterns.
///
/// `BaseLoad` creates a sinusoidal power demand pattern with configurable baseline,
/// amplitude, phase, and random noise to simulate typical daily load patterns.
///
/// # Examples
///
/// ```
/// use vpp_sim::devices::baseload::BaseLoad;
///
/// // Create a baseload with typical parameters
/// let mut load = BaseLoad::new(
///     1.0,   // base_kw - average consumption
///     0.5,   // amp_kw - daily variation
///     0.0,   // phase_rad - no phase shift (minimum at midnight)
///     0.05,  // noise_std - small random variation
///     24,    // steps_per_day - hourly resolution
///     42,    // seed - for reproducible randomness
/// );
///
/// // Get demand at noon
/// let demand = load.power_kw(12);
/// ```
#[derive(Debug, Clone)]
pub struct BaseLoad {
    /// Baseline power consumption in kilowatts
    pub base_kw: f32,

    /// Amplitude of the sinusoidal variation in kilowatts
    pub amp_kw: f32,

    /// Phase offset of the sinusoidal pattern in radians
    pub phase_rad: f32,

    /// Standard deviation of the Gaussian noise in kilowatts
    pub noise_std: f32,

    /// Number of time steps per simulated day
    pub steps_per_day: usize,

    /// Random number generator for noise generation
    rng: StdRng,
}

impl BaseLoad {
    /// Creates a new baseload generator with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `base_kw` - The baseline power consumption in kilowatts
    /// * `amp_kw` - The amplitude of sinusoidal daily variation in kilowatts
    /// * `phase_rad` - The phase offset in radians (0 = minimum at start of day)
    /// * `noise_std` - The standard deviation of Gaussian noise in kilowatts
    /// * `steps_per_day` - The number of time steps per simulated day
    /// * `seed` - Random seed for reproducible noise generation
    ///
    /// # Returns
    ///
    /// A new `BaseLoad` instance configured with the specified parameters
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
}

impl Device for BaseLoad {
    /// Calculates the power demand at a specific time step.
    ///
    /// This method computes the power demand as a combination of:
    /// - A baseline component (`base_kw`)
    /// - A sinusoidal daily pattern with specified amplitude and phase
    /// - Random Gaussian noise with specified standard deviation
    ///
    /// The demand is guaranteed to be non-negative.
    ///
    /// # Arguments
    ///
    /// * `timestep` - The simulation time step
    ///
    /// # Returns
    ///
    /// The power demand in kilowatts at the specified time step
    fn power_kw(&mut self, context: &DeviceContext) -> f32 {
        let day_pos = (context.timestep % self.steps_per_day) as f32 / self.steps_per_day as f32; // [0,1)
        let angle = 2.0 * std::f32::consts::PI * day_pos + self.phase_rad;
        let sinus = angle.sin();

        let noise = gaussian_noise(&mut self.rng, self.noise_std);
        let kw = self.base_kw + self.amp_kw * sinus + noise;
        kw.max(0.0) // no negative demand
    }

    fn device_type(&self) -> &'static str {
        "BaseLoad"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    // Helper function to create a context with just a timestep
    fn ctx(t: usize) -> DeviceContext {
        DeviceContext {
            timestep: t,
            setpoint_kw: None,
        }
    }

    #[test]
    fn test_new_baseload() {
        let load = BaseLoad::new(1.0, 0.5, 0.0, 0.1, 24, 42);
        assert_eq!(load.base_kw, 1.0);
        assert_eq!(load.amp_kw, 0.5);
        assert_eq!(load.phase_rad, 0.0);
        assert_eq!(load.noise_std, 0.1);
        assert_eq!(load.steps_per_day, 24);
    }

    #[test]
    fn test_steps_per_day_minimum() {
        // Should enforce minimum of 1 step per day
        let load = BaseLoad::new(1.0, 0.5, 0.0, 0.1, 0, 42);
        assert_eq!(load.steps_per_day, 1);
    }

    #[test]
    fn test_deterministic_pattern() {
        // With zero noise, demand should be predictable
        let mut load = BaseLoad::new(2.0, 1.0, 0.0, 0.0, 4, 42);

        // At phase 0, first step should be base_kw (since sin(0) = 0)
        assert_eq!(load.power_kw(&ctx(0)), 2.0);

        // At quarter day (π/2), should be base_kw + amp_kw (since sin(π/2) = 1)
        let demand = load.power_kw(&ctx(1));
        assert!((demand - 3.0).abs() < 1e-5);

        // At half day (π), should be base_kw (since sin(π) = 0)
        let demand = load.power_kw(&ctx(2));
        assert!((demand - 2.0).abs() < 1e-5);

        // At 3/4 day (3π/2), should be base_kw - amp_kw (since sin(3π/2) = -1)
        let demand = load.power_kw(&ctx(3));
        assert!((demand - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_phase_shift() {
        // Test with phase shift of π/2
        let mut load = BaseLoad::new(2.0, 1.0, PI / 2.0, 0.0, 4, 42);

        // At phase π/2, first step should be base_kw + amp_kw (since sin(π/2) = 1)
        let demand = load.power_kw(&ctx(0));
        assert!((demand - 3.0).abs() < 1e-5);
    }

    #[test]
    fn test_no_negative_demand() {
        // Configure for potential negative values
        let mut load = BaseLoad::new(0.5, 1.0, 0.0, 0.0, 4, 42);

        // At 3/4 day (3π/2), base_kw - amp_kw would be negative, but should be clamped to 0
        let demand = load.power_kw(&ctx(3));
        assert_eq!(demand, 0.0);
    }

    #[test]
    fn test_random_noise_deterministic() {
        // Same seed should produce same sequence of values
        let mut load1 = BaseLoad::new(1.0, 0.0, 0.0, 0.5, 10, 42);
        let mut load2 = BaseLoad::new(1.0, 0.0, 0.0, 0.5, 10, 42);

        for i in 0..5 {
            assert_eq!(load1.power_kw(&ctx(i)), load2.power_kw(&ctx(i)));
        }
    }

    #[test]
    fn test_different_seeds_produce_different_results() {
        // Different seeds should produce different sequences
        let mut load1 = BaseLoad::new(1.0, 0.0, 0.0, 0.5, 10, 42);
        let mut load2 = BaseLoad::new(1.0, 0.0, 0.0, 0.5, 10, 43);

        let mut all_same = true;
        for i in 0..5 {
            if (load1.power_kw(&ctx(i)) - load2.power_kw(&ctx(i))).abs() > 1e-5 {
                all_same = false;
                break;
            }
        }

        assert!(!all_same);
    }
}
