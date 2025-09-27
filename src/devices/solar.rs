use rand::{Rng, SeedableRng, rngs::StdRng};

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
/// let generation = pv.gen_kw(12);
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
        0.5 * (1.0 - (std::f32::consts::PI * x).cos())
    }

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
