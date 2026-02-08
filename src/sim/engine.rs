//! Simulation engine that orchestrates devices, controller, and power balance.

use crate::devices::{BaseLoad, Battery, Device, DeviceContext, EvCharger, SolarPv};

use super::controller::Controller;
use super::event::DemandResponseEvent;
use super::feeder::Feeder;
use super::power_balance::feeder_net_kw;
use super::types::{SimConfig, StepInput, StepResult, StepState};

/// Simulation engine owning all devices, controller, and configuration.
///
/// Generic over `C: Controller` for static dispatch. Holds typed device
/// fields rather than trait objects since the device set is fixed.
pub struct Engine<C: Controller> {
    config: SimConfig,
    load: BaseLoad,
    pv: SolarPv,
    battery: Battery,
    ev: EvCharger,
    feeder: Feeder,
    controller: C,
    load_forecast: Vec<f32>,
    target_schedule: Vec<f32>,
    dr_event: DemandResponseEvent,
}

impl<C: Controller> Engine<C> {
    /// Creates a new simulation engine.
    ///
    /// # Arguments
    ///
    /// * `config` - Simulation configuration
    /// * `load` - Baseload device
    /// * `pv` - Solar PV device
    /// * `battery` - Battery storage device
    /// * `ev` - EV charger device
    /// * `feeder` - Feeder with import/export limits
    /// * `controller` - Dispatch controller
    /// * `load_forecast` - Per-step load forecast (one day, wraps)
    /// * `target_schedule` - Per-step target feeder load (one day, wraps)
    /// * `dr_event` - Demand response event
    #[expect(clippy::too_many_arguments)]
    pub fn new(
        config: SimConfig,
        load: BaseLoad,
        pv: SolarPv,
        battery: Battery,
        ev: EvCharger,
        feeder: Feeder,
        controller: C,
        load_forecast: Vec<f32>,
        target_schedule: Vec<f32>,
        dr_event: DemandResponseEvent,
    ) -> Self {
        Self {
            config,
            load,
            pv,
            battery,
            ev,
            feeder,
            controller,
            load_forecast,
            target_schedule,
            dr_event,
        }
    }

    /// Executes one simulation timestep and returns the result.
    ///
    /// # Arguments
    ///
    /// * `t` - Timestep index
    ///
    /// # Returns
    ///
    /// A `StepResult` capturing all device outputs, dispatch decisions,
    /// feeder balance, and tracking error.
    pub fn step(&mut self, t: usize) -> StepResult {
        let context = DeviceContext::new(t);
        let spd = self.config.steps_per_day;

        // 1. Read device states
        let base_demand_raw_kw = self.load.power_kw(&context);
        let solar_kw = self.pv.power_kw(&context); // negative during daylight
        let ev_requested_kw = self.ev.requested_power_kw(&context);

        let forecast_kw = self.load_forecast[t % spd];
        let target_kw = self.target_schedule[t % spd];
        let dr_requested_kw = self.dr_event.requested_reduction_at_kw(t);

        // 2. Build controller inputs
        let input = StepInput {
            timestep: t,
            forecast_kw,
            target_kw,
            dr_requested_kw,
            base_demand_raw_kw,
            solar_kw,
            ev_requested_kw,
        };

        let state = StepState {
            battery_soc: self.battery.soc,
            battery_max_charge_kw: self.battery.max_charge_kw,
            battery_max_discharge_kw: self.battery.max_discharge_kw,
            max_import_kw: self.feeder.max_import_kw(),
            max_export_kw: self.feeder.max_export_kw(),
        };

        // 3. Controller dispatch
        let dispatch = self.controller.dispatch(&input, &state);

        // 4. Apply dispatch to devices
        let ev_context = DeviceContext::with_setpoint(t, dispatch.ev_cap_kw);
        let ev_actual_kw = self.ev.power_kw(&ev_context);

        let battery_context = DeviceContext::with_setpoint(t, dispatch.battery_setpoint_kw);
        let battery_actual_kw = self.battery.power_kw(&battery_context);

        // 5. Feeder balance (all inputs in feeder convention, no sign flipping)
        let feeder_kw = feeder_net_kw(
            dispatch.base_demand_kw,
            ev_actual_kw,
            solar_kw,
            battery_actual_kw,
        );

        // 6. Check feeder limits
        self.feeder.reset();
        self.feeder.add_net_kw(feeder_kw);
        let within_feeder_limits = self.feeder.within_limits();

        // 7. Build result
        let tracking_error_kw = feeder_kw - target_kw;

        StepResult {
            timestep: t,
            time_hr: t as f32 * self.config.dt_hours,
            base_kw_raw: base_demand_raw_kw,
            base_kw_after_dr: dispatch.base_demand_kw,
            solar_kw,
            ev_requested_kw,
            ev_after_dr_kw: dispatch.ev_after_dr_kw,
            ev_cap_kw: dispatch.ev_cap_kw,
            ev_actual_kw,
            battery_setpoint_kw: dispatch.battery_setpoint_kw,
            battery_actual_kw,
            battery_soc: self.battery.soc,
            feeder_kw,
            target_kw,
            tracking_error_kw,
            dr_requested_kw,
            dr_achieved_kw: dispatch.dr_achieved_kw,
            within_feeder_limits,
        }
    }

    /// Executes all timesteps and returns the complete step record vector.
    pub fn run(&mut self) -> Vec<StepResult> {
        let total = self.config.total_steps();
        let mut results = Vec::with_capacity(total);
        for t in 0..total {
            results.push(self.step(t));
        }
        results
    }

    /// Returns a reference to the battery (for KPI capacity queries).
    pub fn battery(&self) -> &Battery {
        &self.battery
    }

    /// Returns a reference to the simulation configuration.
    pub fn config(&self) -> &SimConfig {
        &self.config
    }
}
