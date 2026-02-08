//! Common types and traits for device simulation components.

use std::f32::consts::PI;

use rand::{Rng, rngs::StdRng};

/// Contextual information passed to devices during power calculations.
/// Includes the current timestep and optional setpoints for controllable devices.
/// # Fields
/// * `timestep` - Current simulation timestep
/// * `setpoint_kw` - Optional power setpoint for controllable devices (kW)
pub struct DeviceContext {
    /// Current simulation timestep index.
    pub timestep: usize,
    /// Optional power setpoint for controllable devices (kW).
    pub setpoint_kw: Option<f32>,
}

impl DeviceContext {
    /// Creates a new `DeviceContext` with the given timestep and no setpoint.
    pub fn new(timestep: usize) -> Self {
        Self {
            timestep,
            setpoint_kw: None,
        }
    }

    /// Creates a new `DeviceContext` with the given timestep and setpoint.
    pub fn with_setpoint(timestep: usize, setpoint_kw: f32) -> Self {
        Self {
            timestep,
            setpoint_kw: Some(setpoint_kw),
        }
    }
}

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
    /// * `context` - Contextual information about the device and simulation state, like:
    ///  - `timestep`: Current simulation time step
    ///  - `setpoint_kw`: Optional power setpoint for controllable devices
    ///
    /// # Returns
    ///
    /// Power in kilowatts (kW) at the specified time step
    fn power_kw(&mut self, context: &DeviceContext) -> f32;

    /// Returns a human-readable type name for the device.
    fn device_type(&self) -> &'static str;
}

/// Calculates the daylight fraction for a specific time step.
///
/// Returns a value between 0.0 and 1.0 representing the relative
/// solar intensity, following a half-cosine shape from sunrise to sunset.
///
/// # Arguments
///
/// * `t` - Current timestep index
/// * `steps_per_day` - Number of timesteps per simulated day
/// * `sunrise_idx` - Time step index when sunrise occurs (inclusive)
/// * `sunset_idx` - Time step index when sunset occurs (exclusive)
///
/// # Returns
///
/// Solar intensity fraction (0.0 at night, up to ~1.0 at solar noon)
pub fn daylight_frac(t: usize, steps_per_day: usize, sunrise_idx: usize, sunset_idx: usize) -> f32 {
    let day_t = t % steps_per_day;
    if day_t < sunrise_idx || day_t >= sunset_idx {
        return 0.0;
    }
    let span = (sunset_idx - sunrise_idx) as f32;
    let x = (day_t - sunrise_idx) as f32 / span;
    0.5 * (1.0 - (2.0 * PI * x).cos())
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
    let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
    z0 * std_dev
}
