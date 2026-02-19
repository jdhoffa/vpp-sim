use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

pub const TELEMETRY_SCHEMA_V1_HEADER: &str = "timestep,time_hr,target_kw,feeder_kw,tracking_error_kw,baseload_kw,solar_kw,ev_requested_kw,ev_dispatched_kw,battery_kw,battery_soc,dr_requested_kw,dr_achieved_kw,limit_ok";

#[derive(Clone, Debug)]
pub struct TelemetryRow {
    pub timestep: usize,
    pub time_hr: f32,
    pub target_kw: f32,
    pub feeder_kw: f32,
    pub tracking_error_kw: f32,
    pub baseload_kw: f32,
    pub solar_kw: f32,
    pub ev_requested_kw: f32,
    pub ev_dispatched_kw: f32,
    pub battery_kw: f32,
    pub battery_soc: f32,
    pub dr_requested_kw: f32,
    pub dr_achieved_kw: f32,
    pub limit_ok: bool,
}

pub fn write_telemetry_csv<W: Write>(writer: &mut W, rows: &[TelemetryRow]) -> io::Result<()> {
    writeln!(writer, "{TELEMETRY_SCHEMA_V1_HEADER}")?;
    for row in rows {
        writeln!(
            writer,
            "{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{}",
            row.timestep,
            row.time_hr,
            row.target_kw,
            row.feeder_kw,
            row.tracking_error_kw,
            row.baseload_kw,
            row.solar_kw,
            row.ev_requested_kw,
            row.ev_dispatched_kw,
            row.battery_kw,
            row.battery_soc,
            row.dr_requested_kw,
            row.dr_achieved_kw,
            row.limit_ok
        )?;
    }
    Ok(())
}

pub fn write_telemetry_to_path(path: &Path, rows: &[TelemetryRow]) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_telemetry_csv(&mut writer, rows)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::{TELEMETRY_SCHEMA_V1_HEADER, write_telemetry_csv};
    use crate::simulation::run_demo_simulation;

    #[test]
    fn telemetry_csv_has_schema_v1_header_and_rows_per_timestep() {
        let result = run_demo_simulation(false);
        assert_eq!(result.telemetry.len(), 24);

        let mut out = Vec::new();
        write_telemetry_csv(&mut out, &result.telemetry).expect("csv export should succeed");

        let csv = String::from_utf8(out).expect("csv output should be valid UTF-8");
        let mut lines = csv.lines();
        assert_eq!(lines.next(), Some(TELEMETRY_SCHEMA_V1_HEADER));
        assert_eq!(lines.count(), 24);
    }

    #[test]
    fn telemetry_export_is_deterministic_for_fixed_seed_and_config() {
        let run_a = run_demo_simulation(false);
        let run_b = run_demo_simulation(false);

        let mut out_a = Vec::new();
        write_telemetry_csv(&mut out_a, &run_a.telemetry).expect("first export should succeed");

        let mut out_b = Vec::new();
        write_telemetry_csv(&mut out_b, &run_b.telemetry).expect("second export should succeed");

        assert_eq!(out_a, out_b);
    }
}
