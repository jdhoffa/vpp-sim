//! Core simulation types: configuration, step data, and controller contracts.

use std::fmt;

/// Centralized simulation configuration.
///
/// All devices and the engine reference this struct for timing parameters,
/// eliminating duplicated `dt_hours` computations.
///
/// # Examples
///
/// ```
/// use vpp_sim::sim::types::SimConfig;
///
/// let cfg = SimConfig::new(24, 1, 42);
/// assert_eq!(cfg.dt_hours, 1.0);
/// assert_eq!(cfg.total_steps(), 24);
/// ```
#[derive(Debug, Clone)]
pub struct SimConfig {
    /// Number of simulation steps per day.
    pub steps_per_day: usize,
    /// Number of days to simulate.
    pub days: usize,
    /// Duration of one timestep in hours, derived as `24.0 / steps_per_day`.
    pub dt_hours: f32,
    /// Master random seed for reproducibility.
    pub seed: u64,
}

impl SimConfig {
    /// Creates a new simulation configuration.
    ///
    /// # Arguments
    ///
    /// * `steps_per_day` - Number of timesteps per simulated day (must be > 0)
    /// * `days` - Number of days to simulate (must be > 0)
    /// * `seed` - Master random seed
    ///
    /// # Panics
    ///
    /// Panics if `steps_per_day` or `days` is zero.
    pub fn new(steps_per_day: usize, days: usize, seed: u64) -> Self {
        assert!(steps_per_day > 0, "steps_per_day must be > 0");
        assert!(days > 0, "days must be > 0");
        Self {
            steps_per_day,
            days,
            dt_hours: 24.0 / steps_per_day as f32,
            seed,
        }
    }

    /// Total number of simulation steps across all days.
    pub fn total_steps(&self) -> usize {
        self.steps_per_day * self.days
    }
}

/// Device readings and external signals for one timestep, fed to the controller.
#[derive(Debug, Clone)]
pub struct StepInput {
    /// Current simulation timestep index.
    pub timestep: usize,
    /// Forecasted load for this timestep (kW).
    pub forecast_kw: f32,
    /// Target feeder net load for this timestep (kW).
    pub target_kw: f32,
    /// Demand response reduction requested at this timestep (kW, >= 0).
    pub dr_requested_kw: f32,
    /// Raw baseload demand before DR curtailment (kW, positive).
    pub base_demand_raw_kw: f32,
    /// Solar generation in feeder convention (kW, negative during daylight).
    pub solar_kw: f32,
    /// Unconstrained EV charging request (kW, positive).
    pub ev_requested_kw: f32,
}

/// Battery and feeder constraints available to the controller.
#[derive(Debug, Clone)]
pub struct StepState {
    /// Current battery state of charge (0.0 to 1.0).
    pub battery_soc: f32,
    /// Maximum battery charging power (kW, positive magnitude).
    pub battery_max_charge_kw: f32,
    /// Maximum battery discharging power (kW, positive magnitude).
    pub battery_max_discharge_kw: f32,
    /// Feeder maximum import power (kW, positive).
    pub max_import_kw: f32,
    /// Feeder maximum export power (kW, positive magnitude).
    pub max_export_kw: f32,
}

/// Controller dispatch decisions for one timestep.
#[derive(Debug, Clone)]
pub struct StepDispatch {
    /// Baseload after demand response curtailment (kW, positive).
    pub base_demand_kw: f32,
    /// EV charging after demand response shed (kW, positive).
    pub ev_after_dr_kw: f32,
    /// EV charging cap after feeder constraint (kW, positive).
    pub ev_cap_kw: f32,
    /// Battery setpoint in feeder convention (kW; positive=charge, negative=discharge).
    pub battery_setpoint_kw: f32,
    /// Achieved demand response reduction (kW, >= 0).
    pub dr_achieved_kw: f32,
}

/// Complete record of one simulation timestep.
#[derive(Debug, Clone)]
pub struct StepResult {
    /// Timestep index.
    pub timestep: usize,
    /// Simulation time in hours.
    pub time_hr: f32,
    /// Raw baseload demand before DR (kW, positive).
    pub base_kw_raw: f32,
    /// Baseload after DR curtailment (kW, positive).
    pub base_kw_after_dr: f32,
    /// Solar power in feeder convention (kW, negative during daylight).
    pub solar_kw: f32,
    /// Unconstrained EV charging request (kW, positive).
    pub ev_requested_kw: f32,
    /// EV charging after DR shed (kW, positive).
    pub ev_after_dr_kw: f32,
    /// EV charging cap from controller (kW, positive).
    pub ev_cap_kw: f32,
    /// Actual EV charging power delivered (kW, positive).
    pub ev_actual_kw: f32,
    /// Battery setpoint from controller (kW; positive=charge, negative=discharge).
    pub battery_setpoint_kw: f32,
    /// Actual battery power in feeder convention (kW; positive=charge, negative=discharge).
    pub battery_actual_kw: f32,
    /// Battery SOC after this step (0.0 to 1.0).
    pub battery_soc: f32,
    /// Feeder net load (kW; positive=import, negative=export).
    pub feeder_kw: f32,
    /// Target feeder net load (kW).
    pub target_kw: f32,
    /// Tracking error: `feeder_kw - target_kw`.
    pub tracking_error_kw: f32,
    /// DR reduction requested (kW).
    pub dr_requested_kw: f32,
    /// DR reduction achieved (kW).
    pub dr_achieved_kw: f32,
    /// Whether feeder net load is within import/export limits.
    pub within_feeder_limits: bool,
}

impl fmt::Display for StepResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "t={:>3} ({:>5.1}h) | feeder={:>6.2} kW  target={:>6.2} kW  \
             err={:>6.2} kW | base={:.2}  solar={:.2}  ev={:.2}  bat={:.2} \
             (SoC={:.1}%) | DR(req={:.2}, done={:.2}) ok={}",
            self.timestep,
            self.time_hr,
            self.feeder_kw,
            self.target_kw,
            self.tracking_error_kw,
            self.base_kw_after_dr,
            self.solar_kw,
            self.ev_actual_kw,
            self.battery_actual_kw,
            self.battery_soc * 100.0,
            self.dr_requested_kw,
            self.dr_achieved_kw,
            self.within_feeder_limits,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_config_basic() {
        let cfg = SimConfig::new(24, 1, 42);
        assert_eq!(cfg.steps_per_day, 24);
        assert_eq!(cfg.days, 1);
        assert_eq!(cfg.dt_hours, 1.0);
        assert_eq!(cfg.seed, 42);
        assert_eq!(cfg.total_steps(), 24);
    }

    #[test]
    fn sim_config_multi_day() {
        let cfg = SimConfig::new(48, 3, 0);
        assert_eq!(cfg.total_steps(), 144);
        assert_eq!(cfg.dt_hours, 0.5);
    }

    #[test]
    #[should_panic]
    fn sim_config_zero_steps_panics() {
        SimConfig::new(0, 1, 0);
    }

    #[test]
    #[should_panic]
    fn sim_config_zero_days_panics() {
        SimConfig::new(24, 0, 0);
    }

    #[test]
    fn step_result_display_does_not_panic() {
        let r = StepResult {
            timestep: 0,
            time_hr: 0.0,
            base_kw_raw: 1.0,
            base_kw_after_dr: 0.9,
            solar_kw: -2.5,
            ev_requested_kw: 3.0,
            ev_after_dr_kw: 2.5,
            ev_cap_kw: 2.5,
            ev_actual_kw: 2.5,
            battery_setpoint_kw: -1.0,
            battery_actual_kw: -1.0,
            battery_soc: 0.48,
            feeder_kw: -0.1,
            target_kw: 0.0,
            tracking_error_kw: -0.1,
            dr_requested_kw: 0.5,
            dr_achieved_kw: 0.5,
            within_feeder_limits: true,
        };
        let s = format!("{r}");
        assert!(!s.is_empty());
    }
}
