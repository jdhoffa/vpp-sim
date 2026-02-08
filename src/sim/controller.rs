//! Controller trait and naive real-time implementation.

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
        let (base_demand_kw, ev_after_dr_kw, dr_achieved_kw) = Self::apply_demand_response_kw(
            input.base_demand_raw_kw,
            input.ev_requested_kw,
            input.dr_requested_kw,
        );

        // 2. Net fixed loads in feeder convention (solar is already negative)
        let net_fixed_kw = base_demand_kw + input.solar_kw;

        // 3. Cap EV charging so feeder import stays feasible with battery help
        let ev_cap_kw = Self::capped_flexible_load_kw(
            net_fixed_kw,
            ev_after_dr_kw,
            state.max_import_kw,
            state.battery_max_discharge_kw,
        );

        // 4. Net load without battery
        let net_without_battery_kw = net_fixed_kw + ev_cap_kw;

        // 5. Battery setpoint to track target, respecting all constraints
        let battery_setpoint_kw = Self::constrained_battery_setpoint_kw(
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

impl NaiveRtController {
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

    #[test]
    fn discharges_when_load_above_target() {
        // net_without_battery = 3.0, target = 1.0
        // battery = target - net = 1.0 - 3.0 = -2.0 (discharge in feeder convention)
        let input = make_input(3.0, 0.0, 0.0, 0.0, 1.0);
        let state = make_state(4.0, 3.0, 5.0, 4.0);
        let d = NaiveRtController.dispatch(&input, &state);
        assert!((d.battery_setpoint_kw - (-2.0)).abs() < 1e-6);
    }

    #[test]
    fn charges_when_load_below_target() {
        // net_without_battery = 1.0, target = 2.5
        // battery = 2.5 - 1.0 = 1.5 (charge in feeder convention)
        let input = make_input(1.0, 0.0, 0.0, 0.0, 2.5);
        let state = make_state(4.0, 3.0, 5.0, 4.0);
        let d = NaiveRtController.dispatch(&input, &state);
        assert!((d.battery_setpoint_kw - 1.5).abs() < 1e-6);
    }

    #[test]
    fn caps_flexible_load_when_import_cannot_be_met() {
        // net_fixed = 6.0, ev_req = 4.0, max_import = 5.0, bat_discharge = 3.0
        // overload = 6 + 4 - 3 - 5 = 2 → capped = 4 - 2 = 2
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
        // net = -6.0, target = -5.0, max_export = 2.0
        // desired battery = -5.0 - (-6.0) = 1.0 (charge)
        // but feeder = -6 + battery ≥ -2 → battery ≥ 4.0
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
}
