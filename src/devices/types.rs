//! Common types and traits for device simulation components.

use rand::{Rng, rngs::StdRng};

/// Trait defining a device that can produce or consume electricity.
///
/// This trait provides a common interface for all devices in the simulation,
/// allowing them to be used interchangeably in power flow calculations.
pub trait Device {
    /// Returns the power value at the specified time step.
    ///
    /// Positive values indicate power consumption (load),
    /// negative values indicate power generation.
    ///
    /// # Arguments
    ///
    /// * `timestep` - The simulation time step
    ///
    /// # Returns
    ///
    /// Power in kilowatts (kW) at the specified time step
    fn power_kw(&mut self, timestep: usize) -> f32;

    /// Returns a human-readable type name for the device.
    fn device_type(&self) -> &'static str;
}

/// Utility function to generate Gaussian noise using Box-Muller transform.
///
/// # Arguments
///
/// * `rng` - Random number generator
/// * `std_dev` - Standard deviation of the noise
///
/// # Returns
///
/// Random value from a Gaussian distribution with mean 0 and specified standard deviation
pub fn gaussian_noise(rng: &mut StdRng, std_dev: f32) -> f32 {
    if std_dev <= 0.0 {
        return 0.0;
    }

    let u1: f32 = rng.random::<f32>().clamp(1e-6, 1.0);
    let u2: f32 = rng.random::<f32>();
    let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos();
    z0 * std_dev
}
