/// Naive real-time controller.
///
/// Uses only the battery to track a target feeder net load.
#[derive(Debug, Default, Clone, Copy)]
pub struct NaiveRtController;

impl NaiveRtController {
    /// Compute the battery power setpoint required to track target feeder load.
    ///
    /// Feeder model: `feeder_kw = net_without_battery - battery_kw`
    /// Therefore: `battery_kw = net_without_battery - target_kw`
    pub fn battery_setpoint_kw(&self, net_without_battery: f32, target_kw: f32) -> f32 {
        net_without_battery - target_kw
    }

    /// Cap a flexible load (e.g., EV charging) so feeder import can be kept under limit
    /// with available battery discharge.
    pub fn capped_flexible_load_kw(
        &self,
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

    /// Compute battery setpoint while enforcing feeder import/export constraints.
    pub fn constrained_battery_setpoint_kw(
        &self,
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

        // From feeder = net_without_battery - battery, derive feasible battery range
        // that respects feeder limits and battery limits.
        let low_kw = (-battery_max_charge_kw).max(net_without_battery_kw - max_feeder_kw);
        let high_kw = battery_max_discharge_kw.min(net_without_battery_kw - min_feeder_kw);

        let desired_kw = net_without_battery_kw - constrained_target_kw;
        if low_kw <= high_kw {
            desired_kw.clamp(low_kw, high_kw)
        } else {
            // No feasible point can satisfy both battery and feeder limits; return the
            // battery-limited command closest to desired.
            desired_kw.clamp(-battery_max_charge_kw, battery_max_discharge_kw)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NaiveRtController;

    #[test]
    fn discharges_when_load_is_above_target() {
        let controller = NaiveRtController;
        let setpoint = controller.battery_setpoint_kw(3.0, 1.0);
        assert_eq!(setpoint, 2.0);
    }

    #[test]
    fn charges_when_load_is_below_target() {
        let controller = NaiveRtController;
        let setpoint = controller.battery_setpoint_kw(1.0, 2.5);
        assert_eq!(setpoint, -1.5);
    }

    #[test]
    fn caps_flexible_load_when_import_cannot_be_met() {
        let controller = NaiveRtController;
        let capped = controller.capped_flexible_load_kw(6.0, 4.0, 5.0, 3.0);
        assert_eq!(capped, 2.0);
    }

    #[test]
    fn keeps_flexible_load_when_import_is_feasible() {
        let controller = NaiveRtController;
        let capped = controller.capped_flexible_load_kw(2.0, 2.5, 5.0, 3.0);
        assert_eq!(capped, 2.5);
    }

    #[test]
    fn constrained_battery_setpoint_respects_import_limit() {
        let controller = NaiveRtController;
        let battery_kw = controller.constrained_battery_setpoint_kw(6.0, 1.0, 5.0, 4.0, 4.0, 3.0);
        let feeder_kw = 6.0 - battery_kw;
        assert!(feeder_kw <= 5.0 + 1e-6);
    }

    #[test]
    fn constrained_battery_setpoint_is_battery_limited_when_infeasible() {
        let controller = NaiveRtController;
        let battery_kw = controller.constrained_battery_setpoint_kw(10.0, 1.0, 5.0, 4.0, 4.0, 3.0);
        let feeder_kw = 10.0 - battery_kw;
        assert_eq!(battery_kw, 3.0);
        assert_eq!(feeder_kw, 7.0);
    }

    #[test]
    fn constrained_battery_setpoint_respects_export_limit() {
        let controller = NaiveRtController;
        let battery_kw = controller.constrained_battery_setpoint_kw(-6.0, -5.0, 5.0, 2.0, 4.0, 3.0);
        let feeder_kw = -6.0 - battery_kw;
        assert!(feeder_kw >= -2.0 - 1e-6);
    }
}
