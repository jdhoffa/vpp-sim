//! API response and query types.
//!
//! Field names follow CSV schema v1 conventions for consistency across
//! export formats.

use serde::{Deserialize, Serialize};

use crate::sim::kpi::KpiReport;
use crate::sim::types::{SimConfig, StepResult};

/// Combined state response: config, KPIs, and latest telemetry record.
#[derive(Debug, Serialize)]
pub struct StateResponse {
    /// Simulation configuration.
    pub config: SimConfig,
    /// Aggregate KPI report.
    pub kpi: KpiReport,
    /// Most recent telemetry record (last timestep).
    pub latest_step: TelemetryRecord,
}

/// Single telemetry record using CSV schema v1 field names.
///
/// Maps internal `StepResult` fields to the public API contract:
/// - `base_kw_after_dr` → `baseload_kw`
/// - `ev_actual_kw` → `ev_dispatched_kw`
/// - `battery_actual_kw` → `battery_kw`
/// - `within_feeder_limits` → `limit_ok`
#[derive(Debug, Serialize)]
pub struct TelemetryRecord {
    /// Timestep index.
    pub timestep: usize,
    /// Simulation time in hours.
    pub time_hr: f32,
    /// Target feeder net load (kW).
    pub target_kw: f32,
    /// Actual feeder net load (kW).
    pub feeder_kw: f32,
    /// Tracking error: `feeder_kw - target_kw`.
    pub tracking_error_kw: f32,
    /// Baseload after DR curtailment (kW).
    pub baseload_kw: f32,
    /// Solar power in feeder convention (kW).
    pub solar_kw: f32,
    /// Unconstrained EV charging request (kW).
    pub ev_requested_kw: f32,
    /// Actual EV charging power delivered (kW).
    pub ev_dispatched_kw: f32,
    /// Actual battery power in feeder convention (kW).
    pub battery_kw: f32,
    /// Battery state of charge (0.0 to 1.0).
    pub battery_soc: f32,
    /// DR reduction requested (kW).
    pub dr_requested_kw: f32,
    /// DR reduction achieved (kW).
    pub dr_achieved_kw: f32,
    /// Whether feeder net load is within limits.
    pub limit_ok: bool,
    /// Imbalance cost for this timestep.
    pub imbalance_cost: f32,
}

impl From<&StepResult> for TelemetryRecord {
    fn from(r: &StepResult) -> Self {
        Self {
            timestep: r.timestep,
            time_hr: r.time_hr,
            target_kw: r.target_kw,
            feeder_kw: r.feeder_kw,
            tracking_error_kw: r.tracking_error_kw,
            baseload_kw: r.base_kw_after_dr,
            solar_kw: r.solar_kw,
            ev_requested_kw: r.ev_requested_kw,
            ev_dispatched_kw: r.ev_actual_kw,
            battery_kw: r.battery_actual_kw,
            battery_soc: r.battery_soc,
            dr_requested_kw: r.dr_requested_kw,
            dr_achieved_kw: r.dr_achieved_kw,
            limit_ok: r.within_feeder_limits,
            imbalance_cost: r.imbalance_cost,
        }
    }
}

/// Optional range query parameters for the telemetry endpoint.
#[derive(Debug, Deserialize)]
pub struct TelemetryQuery {
    /// Start timestep (inclusive).
    pub from: Option<usize>,
    /// End timestep (inclusive).
    pub to: Option<usize>,
}

/// Error response body for 400-class errors.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Human-readable error message.
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_step_result() -> StepResult {
        StepResult {
            timestep: 5,
            time_hr: 5.0,
            base_kw_raw: 1.0,
            base_kw_after_dr: 0.9,
            solar_kw: -2.5,
            ev_requested_kw: 3.0,
            ev_after_dr_kw: 2.5,
            ev_cap_kw: 2.5,
            ev_actual_kw: 2.4,
            battery_setpoint_kw: -1.0,
            battery_actual_kw: -0.95,
            battery_soc: 0.48,
            feeder_kw: -0.15,
            target_kw: 0.0,
            tracking_error_kw: -0.15,
            dr_requested_kw: 0.5,
            dr_achieved_kw: 0.4,
            within_feeder_limits: true,
            imbalance_cost: 0.015,
        }
    }

    #[test]
    fn telemetry_record_from_step_result_maps_fields() {
        let step = make_step_result();
        let record = TelemetryRecord::from(&step);

        assert_eq!(record.timestep, 5);
        assert_eq!(record.time_hr, 5.0);
        assert_eq!(record.target_kw, 0.0);
        assert_eq!(record.feeder_kw, -0.15);
        assert_eq!(record.tracking_error_kw, -0.15);
        // CSV schema v1 renames
        assert_eq!(record.baseload_kw, 0.9); // base_kw_after_dr
        assert_eq!(record.solar_kw, -2.5);
        assert_eq!(record.ev_requested_kw, 3.0);
        assert_eq!(record.ev_dispatched_kw, 2.4); // ev_actual_kw
        assert_eq!(record.battery_kw, -0.95); // battery_actual_kw
        assert_eq!(record.battery_soc, 0.48);
        assert_eq!(record.dr_requested_kw, 0.5);
        assert_eq!(record.dr_achieved_kw, 0.4);
        assert!(record.limit_ok); // within_feeder_limits
        assert_eq!(record.imbalance_cost, 0.015);
    }
}
