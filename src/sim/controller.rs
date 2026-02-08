//! Controller trait, shared dispatch helpers, and controller implementations.

use crate::devices::types::daylight_frac;

use super::types::{StepDispatch, StepInput, StepState};

/// Trait for simulation controllers that compute dispatch decisions.
///
/// A controller receives device readings and constraints for one timestep
/// and returns dispatch setpoints for controllable devices.
pub trait Controller {
    /// Compute dispatch decisions for a single timestep.
    ///
    /// # Arguments
    ///
    /// * `input` - Device readings and external signals
    /// * `state` - Battery and feeder constraints
    ///
    /// # Returns
    ///
    /// Dispatch setpoints for controllable devices
    fn dispatch(&self, input: &StepInput, state: &StepState) -> StepDispatch;
}

// ---------------------------------------------------------------------------
// Shared dispatch helpers (used by both Naive and Greedy controllers)
// ---------------------------------------------------------------------------

/// Apply demand response by shedding flexible load first, then curtailable baseload.
///
/// Returns `(baseload_after_kw, flexible_after_kw, achieved_reduction_kw)`.
fn apply_demand_response_kw(
    baseload_kw: f32,
    flexible_load_kw: f32,
    requested_reduction_kw: f32,
) -> (f32, f32, f32) {
    let mut remaining_reduction = requested_reduction_kw.max(0.0);

    let flexible = flexible_load_kw.max(0.0);
    let flex_shed = flexible.min(remaining_reduction);
    let flexible_after = flexible - flex_shed;
    remaining_reduction -= flex_shed;

    let base = baseload_kw.max(0.0);
    let base_shed = base.min(remaining_reduction);
    let baseload_after = base - base_shed;

    let achieved = flex_shed + base_shed;
    (baseload_after, flexible_after, achieved)
}

/// Cap a flexible load (e.g., EV charging) so feeder import can stay
/// under the limit with available battery discharge.
fn capped_flexible_load_kw(
    net_fixed_kw: f32,
    requested_flexible_kw: f32,
    max_import_kw: f32,
    battery_max_discharge_kw: f32,
) -> f32 {
    let requested = requested_flexible_kw.max(0.0);
    let overload_kw =
        (net_fixed_kw + requested - battery_max_discharge_kw - max_import_kw).max(0.0);
    (requested - overload_kw).max(0.0)
}

/// Compute battery setpoint in feeder convention while enforcing
/// feeder import/export and battery kW constraints.
///
/// Feeder model: `feeder_kw = net_without_battery + battery_kw`
/// Target tracking: `battery_kw = target_kw - net_without_battery_kw`
fn constrained_battery_setpoint_kw(
    net_without_battery_kw: f32,
    target_kw: f32,
    max_import_kw: f32,
    max_export_kw: f32,
    battery_max_charge_kw: f32,
    battery_max_discharge_kw: f32,
) -> f32 {
    let min_feeder_kw = -max_export_kw;
    let max_feeder_kw = max_import_kw;
    let constrained_target_kw = target_kw.clamp(min_feeder_kw, max_feeder_kw);

    // feeder = net + battery → battery = feeder - net
    // Battery feasible in feeder convention: [-max_discharge, +max_charge]
    // Feeder feasible: [min_feeder, max_feeder]
    // Combined: battery ∈ [min_feeder - net, max_feeder - net] ∩ [-max_discharge, max_charge]
    let low_kw = (-battery_max_discharge_kw).max(min_feeder_kw - net_without_battery_kw);
    let high_kw = battery_max_charge_kw.min(max_feeder_kw - net_without_battery_kw);

    let desired_kw = constrained_target_kw - net_without_battery_kw;
    if low_kw <= high_kw {
        desired_kw.clamp(low_kw, high_kw)
    } else {
        // No feasible point satisfies both battery and feeder limits;
        // fall back to battery-limited command closest to desired.
        desired_kw.clamp(-battery_max_discharge_kw, battery_max_charge_kw)
    }
}

/// Compute the battery feasibility window in feeder convention.
///
/// Returns `(low_kw, high_kw)` bounding the feasible battery setpoint.
fn battery_feasibility_window(
    net_without_battery_kw: f32,
    max_import_kw: f32,
    max_export_kw: f32,
    battery_max_charge_kw: f32,
    battery_max_discharge_kw: f32,
) -> (f32, f32) {
    let min_feeder_kw = -max_export_kw;
    let max_feeder_kw = max_import_kw;
    let low = (-battery_max_discharge_kw).max(min_feeder_kw - net_without_battery_kw);
    let high = battery_max_charge_kw.min(max_feeder_kw - net_without_battery_kw);
    (low, high)
}

// ---------------------------------------------------------------------------
// NaiveRtController
// ---------------------------------------------------------------------------

/// Naive real-time controller.
///
/// Preserves the existing dispatch logic: apply demand response to shed
/// flexible load first, cap EV charging to respect feeder import limits,
/// then compute a battery setpoint to track the target feeder load.
///
/// All power values use feeder convention:
/// - Positive = import / load
/// - Negative = export / generation
#[derive(Debug, Default, Clone, Copy)]
pub struct NaiveRtController;

impl Controller for NaiveRtController {
    fn dispatch(&self, input: &StepInput, state: &StepState) -> StepDispatch {
        // 1. Apply demand response: shed EV first, then baseload
        let (base_demand_kw, ev_after_dr_kw, dr_achieved_kw) = apply_demand_response_kw(
            input.base_demand_raw_kw,
            input.ev_requested_kw,
            input.dr_requested_kw,
        );

        // 2. Net fixed loads in feeder convention (solar is already negative)
        let net_fixed_kw = base_demand_kw + input.solar_kw;

        // 3. Cap EV charging so feeder import stays feasible with battery help
        let ev_cap_kw = capped_flexible_load_kw(
            net_fixed_kw,
            ev_after_dr_kw,
            state.max_import_kw,
            state.battery_max_discharge_kw,
        );

        // 4. Net load without battery
        let net_without_battery_kw = net_fixed_kw + ev_cap_kw;

        // 5. Battery setpoint to track target, respecting all constraints
        let battery_setpoint_kw = constrained_battery_setpoint_kw(
            net_without_battery_kw,
            input.target_kw,
            state.max_import_kw,
            state.max_export_kw,
            state.battery_max_charge_kw,
            state.battery_max_discharge_kw,
        );

        StepDispatch {
            base_demand_kw,
            ev_after_dr_kw,
            ev_cap_kw,
            battery_setpoint_kw,
            dr_achieved_kw,
        }
    }
}

// ---------------------------------------------------------------------------
// GreedyController
// ---------------------------------------------------------------------------

/// Greedy heuristic controller with forecast-aware battery dispatch.
///
/// Uses the load forecast and estimated solar profile to anticipate future
/// battery needs. When the total expected charge (or discharge) demand exceeds
/// the available battery capacity, the controller scales down the current-step
/// battery setpoint proportionally. This spreads the limited battery capacity
/// across all steps that need it, trading small errors at many steps for
/// avoided large errors at a few steps. Since RMSE penalizes large errors
/// quadratically, this redistribution reduces overall tracking error.
#[derive(Debug, Clone)]
pub struct GreedyController {
    /// Number of timesteps per day.
    steps_per_day: usize,
    /// Battery capacity (kWh).
    capacity_kwh: f32,
    /// Timestep duration (hours).
    dt_hours: f32,
    /// Charge efficiency.
    eta_c: f32,
    /// Discharge efficiency.
    eta_d: f32,
    /// Cumulative future charge energy needed from step t onward (kWh, one day).
    remaining_charge_kwh: Vec<f32>,
    /// Cumulative future discharge energy needed from step t onward (kWh, one day).
    remaining_discharge_kwh: Vec<f32>,
}

impl GreedyController {
    /// Creates a new greedy controller with precomputed energy-demand lookahead.
    ///
    /// # Arguments
    ///
    /// * `forecast` - One-day load forecast (kW, positive, length = `steps_per_day`)
    /// * `target` - One-day target feeder schedule (kW, same length as forecast)
    /// * `capacity_kwh` - Battery energy capacity
    /// * `max_charge_kw` - Battery max charging power
    /// * `max_discharge_kw` - Battery max discharging power
    /// * `initial_soc` - Starting state of charge (0.0–1.0)
    /// * `eta_c` - Charge efficiency
    /// * `eta_d` - Discharge efficiency
    /// * `dt_hours` - Timestep duration in hours
    /// * `solar_kw_peak` - Solar peak generation (kW)
    /// * `sunrise_idx` - Sunrise timestep index (inclusive)
    /// * `sunset_idx` - Sunset timestep index (exclusive)
    ///
    /// # Panics
    ///
    /// Panics if forecast is empty or forecast and target differ in length.
    #[expect(clippy::too_many_arguments)]
    pub fn new(
        forecast: &[f32],
        target: &[f32],
        capacity_kwh: f32,
        max_charge_kw: f32,
        max_discharge_kw: f32,
        _initial_soc: f32,
        eta_c: f32,
        eta_d: f32,
        dt_hours: f32,
        solar_kw_peak: f32,
        sunrise_idx: usize,
        sunset_idx: usize,
    ) -> Self {
        assert!(!forecast.is_empty(), "forecast must not be empty");
        assert!(
            forecast.len() == target.len(),
            "forecast and target must have same length"
        );

        let steps_per_day = forecast.len();

        // Precompute cumulative future energy demands (reverse prefix sums)
        let mut remaining_charge_kwh = vec![0.0_f32; steps_per_day + 1];
        let mut remaining_discharge_kwh = vec![0.0_f32; steps_per_day + 1];

        for t in (0..steps_per_day).rev() {
            let solar_est =
                Self::estimate_solar_kw(t, steps_per_day, sunrise_idx, sunset_idx, solar_kw_peak);
            let net_est = forecast[t] + solar_est;
            let residual = target[t] - net_est;

            if residual > 0.0 {
                // Battery would charge — accumulate as stored energy (after efficiency)
                let charge_kw = residual.min(max_charge_kw);
                remaining_charge_kwh[t] =
                    remaining_charge_kwh[t + 1] + charge_kw * dt_hours * eta_c;
            } else {
                remaining_charge_kwh[t] = remaining_charge_kwh[t + 1];
            }

            if residual < 0.0 {
                // Battery would discharge — accumulate as energy drawn (before efficiency)
                let discharge_kw = (-residual).min(max_discharge_kw);
                remaining_discharge_kwh[t] =
                    remaining_discharge_kwh[t + 1] + discharge_kw * dt_hours / eta_d;
            } else {
                remaining_discharge_kwh[t] = remaining_discharge_kwh[t + 1];
            }
        }

        Self {
            steps_per_day,
            capacity_kwh,
            dt_hours,
            eta_c,
            eta_d,
            remaining_charge_kwh,
            remaining_discharge_kwh,
        }
    }

    /// Estimates deterministic solar power at a given timestep (feeder convention).
    fn estimate_solar_kw(
        t: usize,
        steps_per_day: usize,
        sunrise: usize,
        sunset: usize,
        kw_peak: f32,
    ) -> f32 {
        -kw_peak * daylight_frac(t, steps_per_day, sunrise, sunset)
    }
}

impl Controller for GreedyController {
    fn dispatch(&self, input: &StepInput, state: &StepState) -> StepDispatch {
        // 1. DR and EV capping: identical to naive controller
        let (base_demand_kw, ev_after_dr_kw, dr_achieved_kw) = apply_demand_response_kw(
            input.base_demand_raw_kw,
            input.ev_requested_kw,
            input.dr_requested_kw,
        );

        let net_fixed_kw = base_demand_kw + input.solar_kw;
        let ev_cap_kw = capped_flexible_load_kw(
            net_fixed_kw,
            ev_after_dr_kw,
            state.max_import_kw,
            state.battery_max_discharge_kw,
        );

        let net_without_battery_kw = net_fixed_kw + ev_cap_kw;

        // 2. Compute naive tracking setpoint
        let min_feeder_kw = -state.max_export_kw;
        let max_feeder_kw = state.max_import_kw;
        let constrained_target = input.target_kw.clamp(min_feeder_kw, max_feeder_kw);
        let tracking_kw = constrained_target - net_without_battery_kw;

        // 3. Capacity-aware rate limiting
        let t_mod = input.timestep % self.steps_per_day;
        let next = t_mod + 1;

        let desired_kw = if tracking_kw > 0.0 {
            // Charging: check if total future charge demand exceeds available room
            let current_energy = tracking_kw * self.dt_hours * self.eta_c;
            let future_energy = self.remaining_charge_kwh[next.min(self.steps_per_day)];
            let total_demand = current_energy + future_energy;
            let available_room = (1.0 - state.battery_soc) * self.capacity_kwh;

            if total_demand > available_room && total_demand > 0.0 {
                let scale = available_room / total_demand;
                tracking_kw * scale
            } else {
                tracking_kw
            }
        } else if tracking_kw < 0.0 {
            // Discharging: check if total future discharge demand exceeds stored energy
            let current_energy = (-tracking_kw) * self.dt_hours / self.eta_d;
            let future_energy = self.remaining_discharge_kwh[next.min(self.steps_per_day)];
            let total_demand = current_energy + future_energy;
            let available_energy = state.battery_soc * self.capacity_kwh;

            if total_demand > available_energy && total_demand > 0.0 {
                let scale = available_energy / total_demand;
                tracking_kw * scale
            } else {
                tracking_kw
            }
        } else {
            0.0
        };

        // 4. Apply feasibility constraints (same window as naive)
        let (low_kw, high_kw) = battery_feasibility_window(
            net_without_battery_kw,
            state.max_import_kw,
            state.max_export_kw,
            state.battery_max_charge_kw,
            state.battery_max_discharge_kw,
        );

        let battery_setpoint_kw = if low_kw <= high_kw {
            desired_kw.clamp(low_kw, high_kw)
        } else {
            desired_kw.clamp(-state.battery_max_discharge_kw, state.battery_max_charge_kw)
        };

        StepDispatch {
            base_demand_kw,
            ev_after_dr_kw,
            ev_cap_kw,
            battery_setpoint_kw,
            dr_achieved_kw,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_input(base_raw: f32, solar: f32, ev_req: f32, dr_req: f32, target: f32) -> StepInput {
        StepInput {
            timestep: 0,
            forecast_kw: target,
            target_kw: target,
            dr_requested_kw: dr_req,
            base_demand_raw_kw: base_raw,
            solar_kw: solar,
            ev_requested_kw: ev_req,
        }
    }

    fn make_state(
        max_charge: f32,
        max_discharge: f32,
        max_import: f32,
        max_export: f32,
    ) -> StepState {
        StepState {
            battery_soc: 0.5,
            battery_max_charge_kw: max_charge,
            battery_max_discharge_kw: max_discharge,
            max_import_kw: max_import,
            max_export_kw: max_export,
        }
    }

    // --- NaiveRtController tests (unchanged) ---

    #[test]
    fn discharges_when_load_above_target() {
        let input = make_input(3.0, 0.0, 0.0, 0.0, 1.0);
        let state = make_state(4.0, 3.0, 5.0, 4.0);
        let d = NaiveRtController.dispatch(&input, &state);
        assert!((d.battery_setpoint_kw - (-2.0)).abs() < 1e-6);
    }

    #[test]
    fn charges_when_load_below_target() {
        let input = make_input(1.0, 0.0, 0.0, 0.0, 2.5);
        let state = make_state(4.0, 3.0, 5.0, 4.0);
        let d = NaiveRtController.dispatch(&input, &state);
        assert!((d.battery_setpoint_kw - 1.5).abs() < 1e-6);
    }

    #[test]
    fn caps_flexible_load_when_import_cannot_be_met() {
        let input = make_input(6.0, 0.0, 4.0, 0.0, 0.0);
        let state = make_state(4.0, 3.0, 5.0, 4.0);
        let d = NaiveRtController.dispatch(&input, &state);
        assert_eq!(d.ev_cap_kw, 2.0);
    }

    #[test]
    fn keeps_flexible_load_when_import_feasible() {
        let input = make_input(2.0, 0.0, 2.5, 0.0, 0.0);
        let state = make_state(4.0, 3.0, 5.0, 4.0);
        let d = NaiveRtController.dispatch(&input, &state);
        assert_eq!(d.ev_cap_kw, 2.5);
    }

    #[test]
    fn constrained_setpoint_respects_import_limit() {
        let input = make_input(6.0, 0.0, 0.0, 0.0, 1.0);
        let state = make_state(4.0, 3.0, 5.0, 4.0);
        let d = NaiveRtController.dispatch(&input, &state);
        let feeder_kw = 6.0 + d.battery_setpoint_kw;
        assert!(feeder_kw <= 5.0 + 1e-6);
    }

    #[test]
    fn constrained_setpoint_battery_limited_when_infeasible() {
        let input = make_input(10.0, 0.0, 0.0, 0.0, 1.0);
        let state = make_state(4.0, 3.0, 5.0, 4.0);
        let d = NaiveRtController.dispatch(&input, &state);
        assert_eq!(d.battery_setpoint_kw, -3.0);
        let feeder_kw = 10.0 + d.battery_setpoint_kw;
        assert_eq!(feeder_kw, 7.0);
    }

    #[test]
    fn constrained_setpoint_respects_export_limit() {
        let input = make_input(0.0, -6.0, 0.0, 0.0, -5.0);
        let state = make_state(4.0, 3.0, 5.0, 2.0);
        let d = NaiveRtController.dispatch(&input, &state);
        let feeder_kw = -6.0 + d.battery_setpoint_kw;
        assert!(feeder_kw >= -2.0 - 1e-6);
    }

    #[test]
    fn demand_response_sheds_flexible_then_baseload() {
        let input = make_input(3.0, 0.0, 2.0, 4.0, 0.0);
        let state = make_state(4.0, 3.0, 5.0, 4.0);
        let d = NaiveRtController.dispatch(&input, &state);
        assert_eq!(d.ev_after_dr_kw, 0.0);
        assert_eq!(d.base_demand_kw, 1.0);
        assert_eq!(d.dr_achieved_kw, 4.0);
    }

    #[test]
    fn demand_response_limited_by_available_load() {
        let input = make_input(1.0, 0.0, 0.5, 3.0, 0.0);
        let state = make_state(4.0, 3.0, 5.0, 4.0);
        let d = NaiveRtController.dispatch(&input, &state);
        assert_eq!(d.ev_after_dr_kw, 0.0);
        assert_eq!(d.base_demand_kw, 0.0);
        assert_eq!(d.dr_achieved_kw, 1.5);
    }

    // --- GreedyController tests ---

    /// Helper: build a greedy controller with baseline-like params.
    fn build_greedy() -> GreedyController {
        let forecast = vec![0.8_f32; 24];
        let target = vec![0.8_f32; 24];
        GreedyController::new(
            &forecast, &target, 10.0, 5.0, 5.0, 0.5, 0.95, 0.95, 1.0, 5.0, 6, 18,
        )
    }

    #[test]
    fn greedy_dr_matches_naive() {
        let greedy = build_greedy();
        let input = make_input(3.0, 0.0, 2.0, 4.0, 0.8);
        let state = make_state(5.0, 5.0, 5.0, 4.0);
        let d = greedy.dispatch(&input, &state);
        assert_eq!(d.ev_after_dr_kw, 0.0);
        assert_eq!(d.base_demand_kw, 1.0);
        assert_eq!(d.dr_achieved_kw, 4.0);
    }

    #[test]
    fn greedy_respects_feeder_import_limit() {
        let greedy = build_greedy();
        let input = make_input(6.0, 0.0, 0.0, 0.0, 1.0);
        let state = make_state(5.0, 5.0, 5.0, 4.0);
        let d = greedy.dispatch(&input, &state);
        let feeder_kw = 6.0 + d.battery_setpoint_kw;
        assert!(feeder_kw <= 5.0 + 1e-6);
    }

    #[test]
    fn greedy_respects_battery_limits() {
        let greedy = build_greedy();
        let input = make_input(10.0, 0.0, 0.0, 0.0, 1.0);
        let state = make_state(5.0, 5.0, 5.0, 4.0);
        let d = greedy.dispatch(&input, &state);
        assert!(d.battery_setpoint_kw >= -5.0 - 1e-6);
        assert!(d.battery_setpoint_kw <= 5.0 + 1e-6);
    }

    #[test]
    fn greedy_throttles_charge_when_capacity_scarce() {
        let greedy = build_greedy();
        // At t=10 (solar), SOC=0.9 (almost full), lots of future solar demand.
        // Greedy should charge less than naive to save room for later.
        let input = StepInput {
            timestep: 10,
            forecast_kw: 0.8,
            target_kw: 0.8,
            dr_requested_kw: 0.0,
            base_demand_raw_kw: 0.4,
            solar_kw: -4.0,
            ev_requested_kw: 0.0,
        };
        let state = StepState {
            battery_soc: 0.9,
            battery_max_charge_kw: 5.0,
            battery_max_discharge_kw: 5.0,
            max_import_kw: 10.0,
            max_export_kw: 10.0,
        };

        let d_naive = NaiveRtController.dispatch(&input, &state);
        let d_greedy = greedy.dispatch(&input, &state);

        // Naive charges as much as possible; greedy throttles
        assert!(
            d_greedy.battery_setpoint_kw < d_naive.battery_setpoint_kw - 0.01,
            "greedy ({:.2}) should charge less than naive ({:.2}) at high SOC",
            d_greedy.battery_setpoint_kw,
            d_naive.battery_setpoint_kw,
        );
    }

    #[test]
    fn greedy_matches_naive_when_no_future_demand() {
        let greedy = build_greedy();
        // At t=22 (night, past sunset), no future charge demand.
        // Greedy should match naive exactly.
        let input = StepInput {
            timestep: 22,
            forecast_kw: 0.8,
            target_kw: 0.8,
            dr_requested_kw: 0.0,
            base_demand_raw_kw: 1.2,
            solar_kw: 0.0,
            ev_requested_kw: 0.0,
        };
        let state = StepState {
            battery_soc: 0.5,
            battery_max_charge_kw: 5.0,
            battery_max_discharge_kw: 5.0,
            max_import_kw: 10.0,
            max_export_kw: 10.0,
        };

        let d_naive = NaiveRtController.dispatch(&input, &state);
        let d_greedy = greedy.dispatch(&input, &state);

        assert!(
            (d_greedy.battery_setpoint_kw - d_naive.battery_setpoint_kw).abs() < 0.01,
            "greedy ({:.2}) should match naive ({:.2}) when no future demand",
            d_greedy.battery_setpoint_kw,
            d_naive.battery_setpoint_kw,
        );
    }

    #[test]
    fn greedy_solar_estimate_negative_during_daylight() {
        for t in 7..17 {
            let solar = GreedyController::estimate_solar_kw(t, 24, 6, 18, 5.0);
            assert!(
                solar < 0.0,
                "solar should be negative at t={t}, got {solar}"
            );
        }
    }

    #[test]
    fn greedy_solar_estimate_zero_at_night() {
        for t in [0, 1, 2, 3, 4, 5, 18, 19, 20, 21, 22, 23] {
            let solar = GreedyController::estimate_solar_kw(t, 24, 6, 18, 5.0);
            assert!(
                (solar).abs() < 1e-6,
                "solar should be ~0 at t={t}, got {solar}"
            );
        }
    }

    #[test]
    #[should_panic]
    fn greedy_panics_on_empty_forecast() {
        GreedyController::new(&[], &[], 10.0, 5.0, 5.0, 0.5, 0.95, 0.95, 1.0, 5.0, 6, 18);
    }

    #[test]
    #[should_panic]
    fn greedy_panics_on_length_mismatch() {
        let forecast = vec![0.8; 24];
        let target = vec![0.8; 12];
        GreedyController::new(
            &forecast, &target, 10.0, 5.0, 5.0, 0.5, 0.95, 0.95, 1.0, 5.0, 6, 18,
        );
    }
}
