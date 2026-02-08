//! Feeder power balance computation.

/// Computes feeder net load from device outputs in feeder convention.
///
/// All inputs must already follow the feeder sign convention:
/// - Positive = import / load (`BaseLoad`, `EvCharger`, Battery charging)
/// - Negative = export / generation (`SolarPv`, Battery discharging)
///
/// This function performs pure summation with **no sign flipping**.
///
/// # Arguments
///
/// * `base_kw` - Baseload demand (positive)
/// * `ev_kw` - EV charger power (positive)
/// * `solar_kw` - Solar PV power (negative during daylight)
/// * `battery_kw` - Battery power (positive=charge, negative=discharge)
///
/// # Returns
///
/// Net feeder load in kW (positive=import, negative=export)
pub fn feeder_net_kw(base_kw: f32, ev_kw: f32, solar_kw: f32, battery_kw: f32) -> f32 {
    base_kw + ev_kw + solar_kw + battery_kw
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_loads_positive() {
        let net = feeder_net_kw(1.0, 2.0, 0.0, 0.0);
        assert_eq!(net, 3.0);
    }

    #[test]
    fn solar_reduces_feeder() {
        let net = feeder_net_kw(1.0, 0.0, -3.0, 0.0);
        assert_eq!(net, -2.0);
    }

    #[test]
    fn battery_discharge_reduces_feeder() {
        let net = feeder_net_kw(2.0, 0.0, 0.0, -1.5);
        assert_eq!(net, 0.5);
    }

    #[test]
    fn mixed_scenario() {
        // base=0.8, ev=3.0, solar=-2.5, battery=-1.0 → 0.3
        let net = feeder_net_kw(0.8, 3.0, -2.5, -1.0);
        assert!((net - 0.3).abs() < 1e-6);
    }
}
