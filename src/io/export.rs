//! CSV export for simulation step results.

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::sim::types::StepResult;

/// Schema v1 column header for CSV telemetry export.
const HEADER: &str = "timestep,time_hr,target_kw,feeder_kw,tracking_error_kw,\
                       baseload_kw,solar_kw,ev_requested_kw,ev_dispatched_kw,\
                       battery_kw,battery_soc,dr_requested_kw,dr_achieved_kw,limit_ok";

/// Exports simulation results to a CSV file at the given path.
///
/// Writes a header row followed by one data row per step using the schema v1
/// column layout. Produces deterministic output for identical inputs.
///
/// # Arguments
///
/// * `results` - Complete simulation step results
/// * `path` - Output file path
///
/// # Errors
///
/// Returns an `io::Error` if file creation or writing fails.
pub fn export_csv(results: &[StepResult], path: &Path) -> io::Result<()> {
    let file = File::create(path)?;
    let buf = io::BufWriter::new(file);
    write_csv(results, buf)
}

/// Writes simulation results as CSV to any writer.
///
/// # Arguments
///
/// * `results` - Complete simulation step results
/// * `writer` - Destination implementing `Write`
///
/// # Errors
///
/// Returns an `io::Error` if writing fails.
pub fn write_csv(results: &[StepResult], writer: impl Write) -> io::Result<()> {
    let mut wtr = csv::WriterBuilder::new().from_writer(writer);

    // Header
    wtr.write_record(HEADER.split(',').map(str::trim))?;

    // Data rows
    for r in results {
        wtr.write_record(&[
            r.timestep.to_string(),
            format!("{:.2}", r.time_hr),
            format!("{:.4}", r.target_kw),
            format!("{:.4}", r.feeder_kw),
            format!("{:.4}", r.tracking_error_kw),
            format!("{:.4}", r.base_kw_after_dr),
            format!("{:.4}", r.solar_kw),
            format!("{:.4}", r.ev_requested_kw),
            format!("{:.4}", r.ev_actual_kw),
            format!("{:.4}", r.battery_actual_kw),
            format!("{:.4}", r.battery_soc),
            format!("{:.4}", r.dr_requested_kw),
            format!("{:.4}", r.dr_achieved_kw),
            r.within_feeder_limits.to_string(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_step(t: usize) -> StepResult {
        StepResult {
            timestep: t,
            time_hr: t as f32,
            base_kw_raw: 1.0,
            base_kw_after_dr: 0.9,
            solar_kw: -2.5,
            ev_requested_kw: 3.0,
            ev_after_dr_kw: 2.5,
            ev_cap_kw: 2.5,
            ev_actual_kw: 2.5,
            battery_setpoint_kw: -1.0,
            battery_actual_kw: -1.0,
            battery_soc: 0.48,
            feeder_kw: -0.1,
            target_kw: 0.0,
            tracking_error_kw: -0.1,
            dr_requested_kw: 0.5,
            dr_achieved_kw: 0.5,
            within_feeder_limits: true,
            imbalance_cost: 0.01,
        }
    }

    #[test]
    fn header_matches_schema_v1() {
        let results = vec![make_step(0)];
        let mut buf = Vec::new();
        write_csv(&results, &mut buf).ok();
        let output = String::from_utf8(buf).ok();
        let first_line = output.as_deref().unwrap_or("").lines().next().unwrap_or("");
        assert_eq!(
            first_line,
            "timestep,time_hr,target_kw,feeder_kw,tracking_error_kw,\
             baseload_kw,solar_kw,ev_requested_kw,ev_dispatched_kw,\
             battery_kw,battery_soc,dr_requested_kw,dr_achieved_kw,limit_ok"
        );
    }

    #[test]
    fn row_count_matches_step_count() {
        let results: Vec<StepResult> = (0..24).map(make_step).collect();
        let mut buf = Vec::new();
        write_csv(&results, &mut buf).ok();
        let output = String::from_utf8(buf).ok();
        let lines: Vec<&str> = output.as_deref().unwrap_or("").lines().collect();
        // 1 header + 24 data rows
        assert_eq!(lines.len(), 25);
    }

    #[test]
    fn deterministic_output() {
        let results: Vec<StepResult> = (0..5).map(make_step).collect();
        let mut buf1 = Vec::new();
        let mut buf2 = Vec::new();
        write_csv(&results, &mut buf1).ok();
        write_csv(&results, &mut buf2).ok();
        assert_eq!(buf1, buf2);
    }

    #[test]
    fn round_trip_parseable() {
        let results: Vec<StepResult> = (0..3).map(make_step).collect();
        let mut buf = Vec::new();
        write_csv(&results, &mut buf).ok();

        let mut rdr = csv::ReaderBuilder::new().from_reader(buf.as_slice());
        let headers = rdr.headers().cloned().ok();
        assert_eq!(headers.as_ref().map(csv::StringRecord::len), Some(14));

        let mut row_count = 0;
        for record in rdr.records() {
            let rec = record.ok();
            assert!(rec.is_some(), "every row should parse");
            let rec = rec.as_ref();
            // Numeric columns parse as f32
            for i in 1..13 {
                let val: Result<f32, _> = rec.unwrap()[i].parse();
                assert!(val.is_ok(), "column {i} should parse as f32");
            }
            // limit_ok parses as bool
            let ok_val: Result<bool, _> = rec.unwrap()[13].parse();
            assert!(ok_val.is_ok(), "limit_ok column should parse as bool");
            row_count += 1;
        }
        assert_eq!(row_count, 3);
    }
}
