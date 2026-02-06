use crate::devices::types::{Device, DeviceContext};
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
#[derive(Debug, Clone)]
pub struct EvCharger {
    /// Maximum charging power in kilowatts.
    pub max_charge_kw: f32,

    /// Number of simulation steps per day.
    pub steps_per_day: usize,

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
    pub fn new(
        max_charge_kw: f32,
        steps_per_day: usize,
        demand_kwh_min: f32,
        demand_kwh_max: f32,
        dwell_steps_min: usize,
        dwell_steps_max: usize,
        seed: u64,
    ) -> Self {
        assert!(max_charge_kw > 0.0);
        assert!(steps_per_day > 0);
        assert!(demand_kwh_min >= 0.0);
        assert!(demand_kwh_max >= demand_kwh_min);
        assert!(dwell_steps_min > 0);
        assert!(dwell_steps_max >= dwell_steps_min);

        Self {
            max_charge_kw,
            steps_per_day,
            demand_kwh_min,
            demand_kwh_max,
            dwell_steps_min,
            dwell_steps_max,
            sampled_day: None,
            session: None,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    fn dt_hours(&self) -> f32 {
        24.0 / self.steps_per_day as f32
    }

    fn sample_session_for_day(&mut self, day: usize) {
        let dwell_max = self.dwell_steps_max.min(self.steps_per_day);
        let dwell_min = self.dwell_steps_min.min(dwell_max);
        let dwell = self.rng.random_range(dwell_min..=dwell_max);

        let latest_arrival = self.steps_per_day - dwell;
        let arrival = self.rng.random_range(0..=latest_arrival);
        let deadline = arrival + dwell;

        let max_deliverable_kwh = self.max_charge_kw * self.dt_hours() * dwell as f32;
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
}

impl Device for EvCharger {
    fn power_kw(&mut self, context: &DeviceContext) -> f32 {
        let day = context.timestep / self.steps_per_day;
        let day_t = context.timestep % self.steps_per_day;
        let dt_hours = self.dt_hours();

        if self.sampled_day != Some(day) {
            self.sample_session_for_day(day);
        }

        let Some(session) = &mut self.session else {
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

        let required_kw = session.remaining_kwh / (remaining_steps as f32 * dt_hours);
        let charge_kw = required_kw.min(self.max_charge_kw).max(0.0);

        let delivered_kwh = charge_kw * dt_hours;
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

    fn ctx(t: usize) -> DeviceContext {
        DeviceContext::new(t)
    }

    #[test]
    fn deterministic_for_same_seed() {
        let mut ev1 = EvCharger::new(7.2, 24, 6.0, 12.0, 4, 10, 42);
        let mut ev2 = EvCharger::new(7.2, 24, 6.0, 12.0, 4, 10, 42);

        for t in 0..48 {
            assert_eq!(ev1.power_kw(&ctx(t)), ev2.power_kw(&ctx(t)));
        }
    }

    #[test]
    fn no_charging_outside_session_window() {
        let mut ev = EvCharger::new(7.2, 24, 0.0, 0.0, 4, 4, 7);

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
        let mut ev = EvCharger::new(7.2, 24, 10.0, 10.0, 6, 6, 99);

        let mut total_kwh = 0.0;
        let dt_hours = 24.0 / 24.0;
        for t in 0..24 {
            total_kwh += ev.power_kw(&ctx(t)) * dt_hours;
        }

        assert!((total_kwh - 10.0).abs() < 1e-4);
    }
}
