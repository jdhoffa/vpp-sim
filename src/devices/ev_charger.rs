use crate::devices::types::{Device, DeviceContext};
use crate::sim::types::SimConfig;
use rand::{Rng, SeedableRng, rngs::StdRng};

#[derive(Debug, Clone)]
struct EvSession {
    arrival_step: usize,
    deadline_step: usize,
    remaining_kwh: f32,
}

/// A flexible EV charging load with random daily arrivals.
///
/// Each simulated day, this model samples one charging session with:
/// - random arrival time
/// - random dwell duration (which sets deadline)
/// - random required energy in kWh
///
/// During an active session, charging power is computed as the minimum required
/// to meet the remaining energy by the deadline, limited by `max_charge_kw`.
///
/// # Power Flow Convention (Feeder)
/// Returns **positive** values (consumption / load on feeder).
#[derive(Debug, Clone)]
pub struct EvCharger {
    /// Maximum charging power in kilowatts.
    pub max_charge_kw: f32,

    /// Number of simulation steps per day.
    steps_per_day: usize,

    /// Duration of one timestep in hours.
    dt_hours: f32,

    /// Minimum daily charging demand in kWh.
    pub demand_kwh_min: f32,

    /// Maximum daily charging demand in kWh.
    pub demand_kwh_max: f32,

    /// Minimum connected duration in simulation steps.
    pub dwell_steps_min: usize,

    /// Maximum connected duration in simulation steps.
    pub dwell_steps_max: usize,

    sampled_day: Option<usize>,
    session: Option<EvSession>,
    rng: StdRng,
}

impl EvCharger {
    /// Creates a new EV charger with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `max_charge_kw` - Maximum charging power in kW (must be > 0)
    /// * `demand_kwh_min` - Minimum daily charging demand in kWh
    /// * `demand_kwh_max` - Maximum daily charging demand in kWh
    /// * `dwell_steps_min` - Minimum connected duration in steps (must be > 0)
    /// * `dwell_steps_max` - Maximum connected duration in steps
    /// * `config` - Simulation configuration for timing
    /// * `seed` - Random seed for reproducible session generation
    ///
    /// # Panics
    ///
    /// Panics if `max_charge_kw` <= 0, demand ranges invalid, or dwell ranges invalid.
    pub fn new(
        max_charge_kw: f32,
        demand_kwh_min: f32,
        demand_kwh_max: f32,
        dwell_steps_min: usize,
        dwell_steps_max: usize,
        config: &SimConfig,
        seed: u64,
    ) -> Self {
        assert!(max_charge_kw > 0.0);
        assert!(demand_kwh_min >= 0.0);
        assert!(demand_kwh_max >= demand_kwh_min);
        assert!(dwell_steps_min > 0);
        assert!(dwell_steps_max >= dwell_steps_min);

        Self {
            max_charge_kw,
            steps_per_day: config.steps_per_day,
            dt_hours: config.dt_hours,
            demand_kwh_min,
            demand_kwh_max,
            dwell_steps_min,
            dwell_steps_max,
            sampled_day: None,
            session: None,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    fn sample_session_for_day(&mut self, day: usize) {
        let dwell_max = self.dwell_steps_max.min(self.steps_per_day);
        let dwell_min = self.dwell_steps_min.min(dwell_max);
        let dwell = self.rng.random_range(dwell_min..=dwell_max);

        let latest_arrival = self.steps_per_day - dwell;
        let arrival = self.rng.random_range(0..=latest_arrival);
        let deadline = arrival + dwell;

        let max_deliverable_kwh = self.max_charge_kw * self.dt_hours * dwell as f32;
        let raw_demand = self
            .rng
            .random_range(self.demand_kwh_min..=self.demand_kwh_max);
        let demand_kwh = raw_demand.min(max_deliverable_kwh).max(0.0);

        self.sampled_day = Some(day);
        self.session = Some(EvSession {
            arrival_step: arrival,
            deadline_step: deadline,
            remaining_kwh: demand_kwh,
        });
    }

    /// Returns the unconstrained charging request at the current timestep.
    pub fn requested_power_kw(&mut self, context: &DeviceContext) -> f32 {
        let day = context.timestep / self.steps_per_day;
        let day_t = context.timestep % self.steps_per_day;

        if self.sampled_day != Some(day) {
            self.sample_session_for_day(day);
        }

        let Some(session) = &self.session else {
            return 0.0;
        };

        if day_t < session.arrival_step || day_t >= session.deadline_step {
            return 0.0;
        }

        if session.remaining_kwh <= 0.0 {
            return 0.0;
        }

        let remaining_steps = session.deadline_step - day_t;
        if remaining_steps == 0 {
            return 0.0;
        }

        (session.remaining_kwh / (remaining_steps as f32 * self.dt_hours)).max(0.0)
    }
}

impl Device for EvCharger {
    /// Returns actual charging power after applying setpoint cap.
    ///
    /// Positive return value (load on feeder).
    fn power_kw(&mut self, context: &DeviceContext) -> f32 {
        let requested_kw = self.requested_power_kw(context);

        if requested_kw <= 0.0 {
            return 0.0;
        }

        let cap_kw = context.setpoint_kw.unwrap_or(self.max_charge_kw).max(0.0);
        let charge_kw = requested_kw.min(cap_kw).min(self.max_charge_kw).max(0.0);

        let Some(session) = &mut self.session else {
            return 0.0;
        };

        let delivered_kwh = charge_kw * self.dt_hours;
        session.remaining_kwh = (session.remaining_kwh - delivered_kwh).max(0.0);

        charge_kw
    }

    fn device_type(&self) -> &'static str {
        "EvCharger"
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
    fn deterministic_for_same_seed() {
        let c = cfg();
        let mut ev1 = EvCharger::new(7.2, 6.0, 12.0, 4, 10, &c, 42);
        let mut ev2 = EvCharger::new(7.2, 6.0, 12.0, 4, 10, &c, 42);

        for t in 0..48 {
            assert_eq!(ev1.power_kw(&ctx(t)), ev2.power_kw(&ctx(t)));
        }
    }

    #[test]
    fn no_charging_outside_session_window() {
        let mut ev = EvCharger::new(7.2, 0.0, 0.0, 4, 4, &cfg(), 7);

        let mut non_zero_steps = 0;
        for t in 0..24 {
            if ev.power_kw(&ctx(t)) > 0.0 {
                non_zero_steps += 1;
            }
        }
        assert_eq!(non_zero_steps, 0);
    }

    #[test]
    fn feasible_session_finishes_by_deadline() {
        let c = cfg();
        let mut ev = EvCharger::new(7.2, 10.0, 10.0, 6, 6, &c, 99);

        let mut total_kwh = 0.0;
        for t in 0..24 {
            total_kwh += ev.power_kw(&ctx(t)) * c.dt_hours;
        }
        assert!((total_kwh - 10.0).abs() < 1e-4);
    }
}
