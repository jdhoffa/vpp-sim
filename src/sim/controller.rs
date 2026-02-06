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
}
