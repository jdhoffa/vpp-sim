use crate::devices::types::{Device, DeviceContext};

/// A battery energy storage system that can charge and discharge electricity.
///
/// `Battery` models a battery with configurable capacity, charge/discharge rates,
/// and efficiencies. It maintains its state of charge (SOC) and enforces operational
/// constraints when given power setpoints.
///
/// # Power Flow Convention
/// - Positive power: Discharging (supplying power to the grid)
/// - Negative power: Charging (consuming power from the grid)
///
/// # Examples
///
/// ```
/// use vpp_sim::devices::battery::Battery;
///
/// // Create a 10kWh battery at 50% SOC with 5kW charge/discharge limits
/// let mut battery = Battery::new(
///     10.0,  // capacity_kwh
///     0.5,   // state of charge (50%)
///     5.0,   // max_charge_kw
///     5.0,   // max_discharge_kw
///     0.95,  // charging efficiency
///     0.95,  // discharging efficiency
///     96,    // steps_per_day (15-min intervals)
/// );
///
/// // Command battery to discharge at 3kW
/// let actual_kw = battery.power_kw(3.0);
///
/// // Command battery to charge at 2kW
/// let actual_kw = battery.power_kw(-2.0);
/// ```
#[derive(Debug, Clone)]
pub struct Battery {
    /// Battery capacity in kilowatt-hours
    pub capacity_kwh: f32,

    /// State of charge as a fraction (0.0 to 1.0)
    pub soc: f32,

    /// Maximum charge power in kilowatts (positive value)
    pub max_charge_kw: f32,

    /// Maximum discharge power in kilowatts (positive value)
    pub max_discharge_kw: f32,

    /// Charging efficiency (0..1.0)
    pub eta_c: f32,

    /// Discharging efficiency (0..1.0)
    pub eta_d: f32,

    /// Number of time steps per day
    pub steps_per_day: usize,
}

impl Battery {
    pub fn new(
        capacity_kwh: f32,
        soc: f32,
        max_charge_kw: f32,
        max_discharge_kw: f32,
        eta_c: f32,
        eta_d: f32,
        steps_per_day: usize,
    ) -> Self {
        assert!(capacity_kwh > 0.0);
        assert!((0.0..=1.0).contains(&soc));
        assert!(max_charge_kw >= 0.0 && max_discharge_kw >= 0.0);
        assert!(eta_c > 0.0 && eta_c <= 1.0);
        assert!(eta_d > 0.0 && eta_d <= 1.0);
        assert!(steps_per_day > 0);

        Self {
            capacity_kwh,
            soc,
            max_charge_kw,
            max_discharge_kw,
            eta_c,
            eta_d,
            steps_per_day,
        }
    }
}

impl Device for Battery {
    /// Returns the actual power output given a power setpoint.
    ///
    /// Takes a setpoint in kW and returns the actual power after accounting for
    /// battery constraints:
    /// - Enforces charge/discharge power limits
    /// - Prevents over-charging or over-discharging
    /// - Updates the battery's state of charge (SOC)
    ///
    /// # Power Convention
    /// - Positive: Battery is discharging (delivering power)
    /// - Negative: Battery is charging (consuming power)
    ///
    /// # Arguments
    ///
    /// * `setpoint_kw` - The requested power setpoint in kW
    ///
    /// # Returns
    ///
    /// The actual power output in kW after applying constraints
    fn power_kw(&mut self, context: &DeviceContext) -> f32 {
        let setpoint_kw = context.setpoint_kw.unwrap_or(0.0);
        let dt_hours = 24.0 / self.steps_per_day as f32;

        // First enforce kW limits
        let cmd_kw = if setpoint_kw >= 0.0 {
            // Discharge (positive)
            setpoint_kw.min(self.max_discharge_kw)
        } else {
            // Charge (negative)
            setpoint_kw.max(-self.max_charge_kw)
        };

        // Enforce SOC limits
        if cmd_kw > 0.0 {
            // Discharge
            let max_kwh_this_step = self.soc * self.capacity_kwh * self.eta_d;
            let max_kw_soc = max_kwh_this_step / dt_hours;
            let actual_kw = cmd_kw.min(max_kw_soc.max(0.0));

            // Update SOC
            self.soc -= (actual_kw * dt_hours) / (self.capacity_kwh * self.eta_d);
            self.soc = self.soc.clamp(0.0, 1.0);

            actual_kw
        } else if cmd_kw < 0.0 {
            // Charge - limit by available capacity
            let cmd_abs = -cmd_kw;
            let max_kwh_this_step = (1.0 - self.soc) * self.capacity_kwh / self.eta_c;
            let max_kw_soc = max_kwh_this_step / dt_hours;
            let actual_abs_kw = cmd_abs.min(max_kw_soc.max(0.0));
            let actual_kw = -actual_abs_kw;

            // Update SOC
            self.soc += (actual_abs_kw * dt_hours * self.eta_c) / self.capacity_kwh;
            self.soc = self.soc.clamp(0.0, 1.0);

            actual_kw
        } else {
            0.0 // No action if setpoint is exactly zero
        }
    }

    fn device_type(&self) -> &'static str {
        "Battery"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_battery() {
        let battery = Battery::new(10.0, 0.5, 5.0, 5.0, 0.95, 0.95, 96);
        assert_eq!(battery.capacity_kwh, 10.0);
        assert_eq!(battery.soc, 0.5);
        assert_eq!(battery.max_charge_kw, 5.0);
        assert_eq!(battery.max_discharge_kw, 5.0);
        assert_eq!(battery.eta_c, 0.95);
        assert_eq!(battery.eta_d, 0.95);
        assert!(battery.steps_per_day == 96);
    }

    #[test]
    #[should_panic]
    fn test_invalid_capacity() {
        Battery::new(0.0, 0.5, 5.0, 5.0, 0.95, 0.95, 96);
    }

    #[test]
    #[should_panic]
    fn test_invalid_soc_high() {
        Battery::new(10.0, 1.1, 5.0, 5.0, 0.95, 0.95, 96);
    }

    #[test]
    #[should_panic]
    fn test_invalid_soc_negative() {
        Battery::new(10.0, -0.1, 5.0, 5.0, 0.95, 0.95, 96);
    }

    #[test]
    fn test_charge_power_limit() {
        let mut battery = Battery::new(10.0, 0.5, 5.0, 5.0, 1.0, 1.0, 96);
        let context = DeviceContext::with_setpoint(0, -10.0);
        let actual_kw = battery.power_kw(&context);
        assert_eq!(actual_kw, -5.0); // Should be limited to -5kW
    }

    #[test]
    fn test_discharge_power_limit() {
        let mut battery = Battery::new(10.0, 0.5, 5.0, 5.0, 1.0, 1.0, 96);
        let context = DeviceContext::with_setpoint(0, 10.0);
        let actual_kw = battery.power_kw(&context);
        assert_eq!(actual_kw, 5.0); // Should be limited to 5kW
    }

    #[test]
    fn test_discharge_soc_limit() {
        // Battery at 10% SOC with 10kWh capacity (= 1kWh available)
        // With 0.25h timestep and perfect efficiency, max discharge is 4kW
        let mut battery = Battery::new(10.0, 0.1, 5.0, 5.0, 1.0, 1.0, 96);

        // Try to discharge at 5kW
        let context = DeviceContext::with_setpoint(0, 5.0);
        let actual_kw = battery.power_kw(&context);
        assert_eq!(actual_kw, 4.0); // Should be limited by available energy

        // SOC should now be 0.0 (fully discharged)
        assert!(battery.soc < 1e-6);
    }

    #[test]
    fn test_charge_soc_limit() {
        // Battery at 90% SOC with 10kWh capacity (= 1kWh available space)
        // With 0.25h timestep and perfect efficiency, max charge is 4kW
        let mut battery = Battery::new(10.0, 0.9, 5.0, 5.0, 1.0, 1.0, 96);

        // Try to charge at 5kW
        let context = DeviceContext::with_setpoint(0, -5.0);
        let actual_kw = battery.power_kw(&context);
        assert!((actual_kw - (-4.0)).abs() < 1e-5); // Should be limited by available capacity

        // SOC should now be 1.0 (fully charged)
        assert!((battery.soc - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_efficiency_charge() {
        // Test charging with losses
        // 10kWh battery at 0% SOC with 90% charging efficiency
        let mut battery = Battery::new(10.0, 0.0, 5.0, 5.0, 0.9, 0.9, 4); // 6h timestep

        // Charge with 1kW for 6 hours = 6kWh
        // Should result in 6kWh * 0.9 = 5.4kWh stored
        let context = DeviceContext::with_setpoint(0, -1.0);
        battery.power_kw(&context);

        // Expected SOC: 5.4kWh / 10kWh = 0.54
        assert!((battery.soc - 0.54).abs() < 1e-6);
    }

    #[test]
    fn test_efficiency_discharge() {
        // Test discharging with losses
        // 10kWh battery at 50% SOC with 80% discharging efficiency
        let mut battery = Battery::new(10.0, 0.5, 5.0, 5.0, 0.9, 0.8, 4); // 6h timestep

        // Discharge with 1kW for 6 hours = 6kWh
        // Should require 6kWh / 0.8 = 7.5kWh from battery
        let context = DeviceContext::with_setpoint(0, 1.0);
        battery.power_kw(&context);

        // Expected SOC: 0.5 - (7.5kWh / 10kWh) = 0.5 - 0.75 = -0.25, clamped to 0.0
        assert_eq!(battery.soc, 0.0);
    }

    #[test]
    fn test_complete_charge_discharge_cycle() {
        // Create a 10kWh battery at 50% SOC
        let mut battery = Battery::new(10.0, 0.5, 2.0, 2.0, 0.9, 0.9, 24); // 1h timestep

        // Fully charge the battery
        while battery.soc < 0.99 {
            let context = DeviceContext::with_setpoint(0, -2.0);
            battery.power_kw(&context);
        }

        // Now fully discharge
        let mut energy_delivered = 0.0;
        while battery.soc > 0.01 {
            let context = DeviceContext::with_setpoint(0, 2.0);
            let kw = battery.power_kw(&context);
            let dt_hours = 24.0 / battery.steps_per_day as f32;
            energy_delivered += kw * dt_hours;
        }

        // We should get approximately 10kWh * 0.9 (discharge efficiency) = 9kWh
        assert!((energy_delivered - 9.0).abs() < 0.1);
    }
}
