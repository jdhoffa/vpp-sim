//! Post-hoc KPI computation from simulation results.

use std::fmt;

use super::types::StepResult;

/// Aggregate key performance indicators derived from a complete simulation run.
///
/// Computed post-hoc from `Vec<StepResult>` to ensure consistency between
/// step data and reported metrics.
#[derive(Debug, Clone)]
pub struct KpiReport {
    /// Root-mean-square tracking error (kW).
    pub rmse_tracking_kw: f32,
    /// Mean absolute tracking error (kW).
    pub mae_tracking_kw: f32,
    /// Percentage of requested DR curtailment achieved.
    pub curtailment_pct: f32,
    /// Peak feeder import power (kW, positive).
    pub peak_import_kw: f32,
    /// Peak feeder export power (kW, positive magnitude).
    pub peak_export_kw: f32,
    /// Total battery energy throughput (kWh, sum of |power| * dt).
    pub battery_throughput_kwh: f32,
    /// Battery equivalent full cycles (throughput / 2*capacity).
    pub battery_equivalent_full_cycles: f32,
    /// Number of timesteps where feeder limits were violated.
    pub feeder_violation_count: usize,
}

impl KpiReport {
    /// Computes all KPIs from the complete step record vector.
    ///
    /// # Arguments
    ///
    /// * `results` - Complete simulation step results
    /// * `dt_hours` - Timestep duration in hours
    /// * `battery_capacity_kwh` - Battery capacity for cycle calculation
    ///
    /// # Returns
    ///
    /// A `KpiReport` with all fields populated.
    pub fn from_results(results: &[StepResult], dt_hours: f32, battery_capacity_kwh: f32) -> Self {
        if results.is_empty() {
            return Self {
                rmse_tracking_kw: 0.0,
                mae_tracking_kw: 0.0,
                curtailment_pct: 0.0,
                peak_import_kw: 0.0,
                peak_export_kw: 0.0,
                battery_throughput_kwh: 0.0,
                battery_equivalent_full_cycles: 0.0,
                feeder_violation_count: 0,
            };
        }

        let n = results.len() as f32;
        let mut sq_sum = 0.0_f32;
        let mut abs_sum = 0.0_f32;
        let mut dr_requested_sum = 0.0_f32;
        let mut dr_achieved_sum = 0.0_f32;
        let mut peak_import = 0.0_f32;
        let mut peak_export = 0.0_f32;
        let mut bat_throughput = 0.0_f32;
        let mut violations = 0_usize;

        for r in results {
            let err = r.tracking_error_kw;
            sq_sum += err * err;
            abs_sum += err.abs();

            dr_requested_sum += r.dr_requested_kw;
            dr_achieved_sum += r.dr_achieved_kw;

            peak_import = peak_import.max(r.feeder_kw);
            peak_export = peak_export.max(-r.feeder_kw);

            bat_throughput += r.battery_actual_kw.abs() * dt_hours;

            if !r.within_feeder_limits {
                violations += 1;
            }
        }

        let curtailment_pct = if dr_requested_sum > 0.0 {
            100.0 * dr_achieved_sum / dr_requested_sum
        } else {
            0.0
        };

        let cycles = if battery_capacity_kwh > 0.0 {
            bat_throughput / (2.0 * battery_capacity_kwh)
        } else {
            0.0
        };

        Self {
            rmse_tracking_kw: (sq_sum / n).sqrt(),
            mae_tracking_kw: abs_sum / n,
            curtailment_pct,
            peak_import_kw: peak_import,
            peak_export_kw: peak_export,
            battery_throughput_kwh: bat_throughput,
            battery_equivalent_full_cycles: cycles,
            feeder_violation_count: violations,
        }
    }
}

impl fmt::Display for KpiReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "--- KPI Report ---")?;
        writeln!(f, "RMSE tracking error:   {:.3} kW", self.rmse_tracking_kw)?;
        writeln!(f, "MAE tracking error:    {:.3} kW", self.mae_tracking_kw)?;
        writeln!(f, "Curtailment achieved:  {:.1}%", self.curtailment_pct)?;
        writeln!(f, "Peak import:           {:.2} kW", self.peak_import_kw)?;
        writeln!(f, "Peak export:           {:.2} kW", self.peak_export_kw)?;
        writeln!(
            f,
            "Battery throughput:    {:.2} kWh ({:.2} equiv. cycles)",
            self.battery_throughput_kwh, self.battery_equivalent_full_cycles
        )?;
        write!(f, "Feeder violations:     {}", self.feeder_violation_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(tracking_error_kw: f32, battery_actual_kw: f32, feeder_kw: f32) -> StepResult {
        StepResult {
            timestep: 0,
            time_hr: 0.0,
            base_kw_raw: 0.0,
            base_kw_after_dr: 0.0,
            solar_kw: 0.0,
            ev_requested_kw: 0.0,
            ev_after_dr_kw: 0.0,
            ev_cap_kw: 0.0,
            ev_actual_kw: 0.0,
            battery_setpoint_kw: 0.0,
            battery_actual_kw,
            battery_soc: 0.5,
            feeder_kw,
            target_kw: feeder_kw - tracking_error_kw,
            tracking_error_kw,
            dr_requested_kw: 0.0,
            dr_achieved_kw: 0.0,
            within_feeder_limits: true,
        }
    }

    #[test]
    fn rmse_computation() {
        // errors: [1.0, -1.0, 2.0, -2.0]
        // sq_sum = 1 + 1 + 4 + 4 = 10, mean = 2.5, sqrt = ~1.581
        let results: Vec<StepResult> = [1.0, -1.0, 2.0, -2.0]
            .iter()
            .map(|&e| make_result(e, 0.0, e))
            .collect();
        let kpi = KpiReport::from_results(&results, 1.0, 10.0);
        assert!((kpi.rmse_tracking_kw - 2.5_f32.sqrt()).abs() < 1e-4);
    }

    #[test]
    fn battery_throughput() {
        // battery powers: [2.0, -3.0, 1.0, -1.0], dt=1.0
        // throughput = 2 + 3 + 1 + 1 = 7.0 kWh
        let results: Vec<StepResult> = [2.0, -3.0, 1.0, -1.0]
            .iter()
            .map(|&b| make_result(0.0, b, 0.0))
            .collect();
        let kpi = KpiReport::from_results(&results, 1.0, 10.0);
        assert!((kpi.battery_throughput_kwh - 7.0).abs() < 1e-4);
    }

    #[test]
    fn feeder_violation_counting() {
        let mut results = vec![make_result(0.0, 0.0, 3.0); 5];
        results[1].within_feeder_limits = false;
        results[3].within_feeder_limits = false;
        let kpi = KpiReport::from_results(&results, 1.0, 10.0);
        assert_eq!(kpi.feeder_violation_count, 2);
    }

    #[test]
    fn peak_import_and_export() {
        let results: Vec<StepResult> = [3.0, -2.0, 5.0, -1.0]
            .iter()
            .map(|&f| make_result(0.0, 0.0, f))
            .collect();
        let kpi = KpiReport::from_results(&results, 1.0, 10.0);
        assert_eq!(kpi.peak_import_kw, 5.0);
        assert_eq!(kpi.peak_export_kw, 2.0);
    }

    #[test]
    fn empty_results() {
        let kpi = KpiReport::from_results(&[], 1.0, 10.0);
        assert_eq!(kpi.rmse_tracking_kw, 0.0);
        assert_eq!(kpi.feeder_violation_count, 0);
    }
}
