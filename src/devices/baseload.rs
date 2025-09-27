use rand::{Rng, SeedableRng, rngs::StdRng};

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
/// let demand = load.demand_kw(12);
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
