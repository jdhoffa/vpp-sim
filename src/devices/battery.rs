use crate::devices::types::{Device, DeviceContext};
use crate::sim::types::SimConfig;

/// A battery energy storage system that can charge and discharge electricity.
///
/// `Battery` models a battery with configurable capacity, charge/discharge rates,
/// and efficiencies. It maintains its state of charge (SOC) and enforces operational
/// constraints when given power setpoints.
///
/// # Power Flow Convention (Feeder)
/// - Positive power: Charging (consuming power from the grid / load)
/// - Negative power: Discharging (supplying power to the grid / generation)
#[derive(Debug, Clone)]
pub struct Battery {
    /// Battery capacity in kilowatt-hours.
    pub capacity_kwh: f32,

    /// State of charge as a fraction (0.0 to 1.0).
    pub soc: f32,

    /// Maximum charge power in kilowatts (positive value).
    pub max_charge_kw: f32,

    /// Maximum discharge power in kilowatts (positive value).
    pub max_discharge_kw: f32,

    /// Charging efficiency (0..1.0).
    pub eta_c: f32,

    /// Discharging efficiency (0..1.0).
    pub eta_d: f32,

    /// Duration of one timestep in hours.
    dt_hours: f32,
}

impl Battery {
    /// Creates a new battery with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `capacity_kwh` - Battery capacity in kWh (must be > 0)
    /// * `soc` - Initial state of charge as a fraction (0.0 to 1.0)
    /// * `max_charge_kw` - Maximum charging power in kW
    /// * `max_discharge_kw` - Maximum discharging power in kW
    /// * `eta_c` - Charging efficiency (0..1.0)
    /// * `eta_d` - Discharging efficiency (0..1.0)
    /// * `config` - Simulation configuration for timing
    ///
    /// # Panics
    ///
    /// Panics if capacity is zero/negative, SOC out of range, or efficiencies invalid.
    pub fn new(
        capacity_kwh: f32,
        soc: f32,
        max_charge_kw: f32,
        max_discharge_kw: f32,
        eta_c: f32,
        eta_d: f32,
        config: &SimConfig,
    ) -> Self {
        assert!(capacity_kwh > 0.0);
        assert!((0.0..=1.0).contains(&soc));
        assert!(max_charge_kw >= 0.0 && max_discharge_kw >= 0.0);
        assert!(eta_c > 0.0 && eta_c <= 1.0);
        assert!(eta_d > 0.0 && eta_d <= 1.0);

        Self {
            capacity_kwh,
            soc,
            max_charge_kw,
            max_discharge_kw,
            eta_c,
            eta_d,
            dt_hours: config.dt_hours,
        }
    }
}

impl Device for Battery {
    /// Returns the actual power output given a power setpoint in feeder convention.
    ///
    /// # Power Convention (Feeder)
    /// - Positive setpoint/return: Charging (load on feeder)
    /// - Negative setpoint/return: Discharging (generation on feeder)
    ///
    /// Enforces charge/discharge power limits, SOC bounds, and efficiency losses.
    fn power_kw(&mut self, context: &DeviceContext) -> f32 {
        let setpoint_kw = context.setpoint_kw.unwrap_or(0.0);

        // Enforce kW limits
        let cmd_kw = if setpoint_kw >= 0.0 {
            // Charge (positive = load on feeder)
            setpoint_kw.min(self.max_charge_kw)
        } else {
            // Discharge (negative = generation on feeder)
            setpoint_kw.max(-self.max_discharge_kw)
        };

        if cmd_kw > 0.0 {
            // Charging — limit by available capacity
            let max_kwh_this_step = (1.0 - self.soc) * self.capacity_kwh / self.eta_c;
            let max_kw_soc = max_kwh_this_step / self.dt_hours;
            let actual_kw = cmd_kw.min(max_kw_soc.max(0.0));

            // Update SOC (increasing)
            self.soc += (actual_kw * self.dt_hours * self.eta_c) / self.capacity_kwh;
            self.soc = self.soc.clamp(0.0, 1.0);

            actual_kw // positive = load on feeder
        } else if cmd_kw < 0.0 {
            // Discharging — limit by available energy
            let cmd_abs = -cmd_kw;
            let max_kwh_this_step = self.soc * self.capacity_kwh * self.eta_d;
            let max_kw_soc = max_kwh_this_step / self.dt_hours;
            let actual_abs_kw = cmd_abs.min(max_kw_soc.max(0.0));

            // Update SOC (decreasing)
            self.soc -= (actual_abs_kw * self.dt_hours) / (self.capacity_kwh * self.eta_d);
            self.soc = self.soc.clamp(0.0, 1.0);

            -actual_abs_kw // negative = generation on feeder
        } else {
            0.0
        }
    }

    fn device_type(&self) -> &'static str {
        "Battery"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(steps_per_day: usize) -> SimConfig {
        SimConfig::new(steps_per_day, 1, 0)
    }

    #[test]
    fn test_new_battery() {
        let c = cfg(96);
        let battery = Battery::new(10.0, 0.5, 5.0, 5.0, 0.95, 0.95, &c);
        assert_eq!(battery.capacity_kwh, 10.0);
        assert_eq!(battery.soc, 0.5);
        assert_eq!(battery.max_charge_kw, 5.0);
        assert_eq!(battery.max_discharge_kw, 5.0);
        assert_eq!(battery.eta_c, 0.95);
        assert_eq!(battery.eta_d, 0.95);
    }

    #[test]
    #[should_panic]
    fn test_invalid_capacity() {
        Battery::new(0.0, 0.5, 5.0, 5.0, 0.95, 0.95, &cfg(96));
    }

    #[test]
    #[should_panic]
    fn test_invalid_soc_high() {
        Battery::new(10.0, 1.1, 5.0, 5.0, 0.95, 0.95, &cfg(96));
    }

    #[test]
    #[should_panic]
    fn test_invalid_soc_negative() {
        Battery::new(10.0, -0.1, 5.0, 5.0, 0.95, 0.95, &cfg(96));
    }

    #[test]
    fn test_charge_power_limit() {
        // Feeder convention: positive setpoint = charge
        let mut battery = Battery::new(10.0, 0.5, 5.0, 5.0, 1.0, 1.0, &cfg(96));
        let context = DeviceContext::with_setpoint(0, 10.0); // request 10kW charge
        let actual_kw = battery.power_kw(&context);
        assert_eq!(actual_kw, 5.0); // limited to 5kW charge (positive)
    }

    #[test]
    fn test_discharge_power_limit() {
        // Feeder convention: negative setpoint = discharge
        let mut battery = Battery::new(10.0, 0.5, 5.0, 5.0, 1.0, 1.0, &cfg(96));
        let context = DeviceContext::with_setpoint(0, -10.0); // request 10kW discharge
        let actual_kw = battery.power_kw(&context);
        assert_eq!(actual_kw, -5.0); // limited to -5kW (discharge)
    }

    #[test]
    fn test_discharge_soc_limit() {
        // Battery at 10% SOC with 10kWh capacity (= 1kWh available)
        // With 0.25h timestep and perfect efficiency, max discharge is 4kW
        let mut battery = Battery::new(10.0, 0.1, 5.0, 5.0, 1.0, 1.0, &cfg(96));
        let context = DeviceContext::with_setpoint(0, -5.0); // request 5kW discharge
        let actual_kw = battery.power_kw(&context);
        assert_eq!(actual_kw, -4.0); // limited by available energy
        assert!(battery.soc < 1e-6);
    }

    #[test]
    fn test_charge_soc_limit() {
        // Battery at 90% SOC with 10kWh capacity (= 1kWh available space)
        // With 0.25h timestep and perfect efficiency, max charge is 4kW
        let mut battery = Battery::new(10.0, 0.9, 5.0, 5.0, 1.0, 1.0, &cfg(96));
        let context = DeviceContext::with_setpoint(0, 5.0); // request 5kW charge
        let actual_kw = battery.power_kw(&context);
        assert!((actual_kw - 4.0).abs() < 1e-5); // limited by available capacity
        assert!((battery.soc - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_efficiency_charge() {
        // 10kWh battery at 0% SOC with 90% charging efficiency
        // 4 steps/day = 6h timestep
        let mut battery = Battery::new(10.0, 0.0, 5.0, 5.0, 0.9, 0.9, &cfg(4));
        // Charge with 1kW for 6 hours = 6kWh from grid
        // Stored: 6kWh * 0.9 = 5.4kWh → SOC = 0.54
        let context = DeviceContext::with_setpoint(0, 1.0); // positive = charge
        battery.power_kw(&context);
        assert!((battery.soc - 0.54).abs() < 1e-6);
    }

    #[test]
    fn test_efficiency_discharge() {
        // 10kWh battery at 50% SOC with 80% discharging efficiency
        let mut battery = Battery::new(10.0, 0.5, 5.0, 5.0, 0.9, 0.8, &cfg(4));
        // Discharge with 1kW for 6 hours = 6kWh delivered
        // Requires 6kWh / 0.8 = 7.5kWh from battery
        // SOC: 0.5 - 7.5/10 = -0.25, clamped to 0.0
        let context = DeviceContext::with_setpoint(0, -1.0); // negative = discharge
        battery.power_kw(&context);
        assert_eq!(battery.soc, 0.0);
    }

    #[test]
    fn test_complete_charge_discharge_cycle() {
        let c = cfg(24); // 1h timestep
        let mut battery = Battery::new(10.0, 0.5, 2.0, 2.0, 0.9, 0.9, &c);

        // Fully charge: positive setpoint
        while battery.soc < 0.99 {
            let context = DeviceContext::with_setpoint(0, 2.0);
            battery.power_kw(&context);
        }

        // Fully discharge: negative setpoint
        let mut energy_delivered = 0.0;
        while battery.soc > 0.01 {
            let context = DeviceContext::with_setpoint(0, -2.0);
            let kw = battery.power_kw(&context);
            energy_delivered += (-kw) * c.dt_hours; // kw is negative, negate to get positive energy
        }

        // Should get ~10kWh * 0.9 (discharge efficiency) = 9kWh
        assert!((energy_delivered - 9.0).abs() < 0.1);
    }
}
