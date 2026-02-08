use std::f32::consts::PI;

use crate::devices::types::{Device, DeviceContext, gaussian_noise};
use crate::sim::types::SimConfig;
use rand::{SeedableRng, rngs::StdRng};

/// A baseload generator that models daily electricity consumption patterns.
///
/// `BaseLoad` creates a sinusoidal power demand pattern with configurable baseline,
/// amplitude, phase, and random noise to simulate typical daily load patterns.
///
/// # Power Flow Convention (Feeder)
/// Returns **positive** values (consumption / load on feeder).
#[derive(Debug, Clone)]
pub struct BaseLoad {
    /// Baseline power consumption in kilowatts.
    pub base_kw: f32,

    /// Amplitude of the sinusoidal variation in kilowatts.
    pub amp_kw: f32,

    /// Phase offset of the sinusoidal pattern in radians.
    pub phase_rad: f32,

    /// Standard deviation of the Gaussian noise in kilowatts.
    pub noise_std: f32,

    /// Number of time steps per simulated day.
    steps_per_day: usize,

    /// Random number generator for noise generation.
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
    /// * `config` - Simulation configuration for timing
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
        config: &SimConfig,
        seed: u64,
    ) -> Self {
        Self {
            base_kw,
            amp_kw,
            phase_rad,
            noise_std,
            steps_per_day: config.steps_per_day.max(1),
            rng: StdRng::seed_from_u64(seed),
        }
    }
}

impl Device for BaseLoad {
    /// Calculates the power demand at a specific time step.
    ///
    /// Computes a sinusoidal pattern with Gaussian noise, clamped to non-negative.
    fn power_kw(&mut self, context: &DeviceContext) -> f32 {
        let day_pos = (context.timestep % self.steps_per_day) as f32 / self.steps_per_day as f32;
        let angle = 2.0 * PI * day_pos + self.phase_rad;
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

    fn cfg(steps_per_day: usize) -> SimConfig {
        SimConfig::new(steps_per_day.max(1), 1, 0)
    }

    fn ctx(t: usize) -> DeviceContext {
        DeviceContext::new(t)
    }

    #[test]
    fn test_new_baseload() {
        let load = BaseLoad::new(1.0, 0.5, 0.0, 0.1, &cfg(24), 42);
        assert_eq!(load.base_kw, 1.0);
        assert_eq!(load.amp_kw, 0.5);
        assert_eq!(load.phase_rad, 0.0);
        assert_eq!(load.noise_std, 0.1);
        assert_eq!(load.steps_per_day, 24);
    }

    #[test]
    fn test_deterministic_pattern() {
        let mut load = BaseLoad::new(2.0, 1.0, 0.0, 0.0, &cfg(4), 42);

        assert_eq!(load.power_kw(&ctx(0)), 2.0);

        let demand = load.power_kw(&ctx(1));
        assert!((demand - 3.0).abs() < 1e-5);

        let demand = load.power_kw(&ctx(2));
        assert!((demand - 2.0).abs() < 1e-5);

        let demand = load.power_kw(&ctx(3));
        assert!((demand - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_phase_shift() {
        let mut load = BaseLoad::new(2.0, 1.0, PI / 2.0, 0.0, &cfg(4), 42);
        let demand = load.power_kw(&ctx(0));
        assert!((demand - 3.0).abs() < 1e-5);
    }

    #[test]
    fn test_no_negative_demand() {
        let mut load = BaseLoad::new(0.5, 1.0, 0.0, 0.0, &cfg(4), 42);
        let demand = load.power_kw(&ctx(3));
        assert_eq!(demand, 0.0);
    }

    #[test]
    fn test_random_noise_deterministic() {
        let c = cfg(10);
        let mut load1 = BaseLoad::new(1.0, 0.0, 0.0, 0.5, &c, 42);
        let mut load2 = BaseLoad::new(1.0, 0.0, 0.0, 0.5, &c, 42);

        for i in 0..5 {
            assert_eq!(load1.power_kw(&ctx(i)), load2.power_kw(&ctx(i)));
        }
    }

    #[test]
    fn test_different_seeds_produce_different_results() {
        let c = cfg(10);
        let mut load1 = BaseLoad::new(1.0, 0.0, 0.0, 0.5, &c, 42);
        let mut load2 = BaseLoad::new(1.0, 0.0, 0.0, 0.5, &c, 43);

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
