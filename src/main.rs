mod devices;
mod forecast;
mod sim;

use devices::{BaseLoad, Battery, Device, DeviceContext, EvCharger, SolarPv};
use forecast::NaiveForecast;
use sim::clock::Clock;
use sim::controller::NaiveRtController;
use sim::event::DemandResponseEvent;
use sim::feeder::Feeder;
use sim::schedule::DayAheadSchedule;
use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

const TELEMETRY_SCHEMA_V1_HEADER: &str = "timestep,time_hr,target_kw,feeder_kw,tracking_error_kw,baseload_kw,solar_kw,ev_requested_kw,ev_dispatched_kw,battery_kw,battery_soc,dr_requested_kw,dr_achieved_kw,limit_ok";

struct CliOptions {
    telemetry_out: Option<PathBuf>,
}

struct SimulationKpis {
    rmse_tracking_kw: f32,
    curtailment_pct: f32,
    feeder_peak_load_kw: f32,
}

struct SimulationResult {
    telemetry: Vec<TelemetryRow>,
    kpis: SimulationKpis,
}

#[derive(Clone, Debug)]
struct TelemetryRow {
    timestep: usize,
    time_hr: f32,
    target_kw: f32,
    feeder_kw: f32,
    tracking_error_kw: f32,
    baseload_kw: f32,
    solar_kw: f32,
    ev_requested_kw: f32,
    ev_dispatched_kw: f32,
    battery_kw: f32,
    battery_soc: f32,
    dr_requested_kw: f32,
    dr_achieved_kw: f32,
    limit_ok: bool,
}

fn parse_args() -> Result<CliOptions, String> {
    let mut args = env::args().skip(1);
    let mut telemetry_out = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--telemetry-out" => {
                let path = args.next().ok_or_else(|| {
                    "missing value for --telemetry-out (expected a file path)".to_string()
                })?;
                if telemetry_out.replace(PathBuf::from(path)).is_some() {
                    return Err("--telemetry-out provided more than once".to_string());
                }
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                return Err(format!("unknown argument: {arg}"));
            }
        }
    }

    Ok(CliOptions { telemetry_out })
}

fn print_usage() {
    eprintln!("Usage: cargo run --release -- [--telemetry-out <path>]");
}

fn write_telemetry_csv<W: Write>(writer: &mut W, rows: &[TelemetryRow]) -> io::Result<()> {
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

fn write_telemetry_to_path(path: &Path, rows: &[TelemetryRow]) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_telemetry_csv(&mut writer, rows)?;
    writer.flush()
}

fn run_demo_simulation(print_readable_log: bool) -> SimulationResult {
    let steps_per_day = 24; // 1-hr intervals
    let dt_hr = 24.0 / steps_per_day as f32;
    let mut clock = Clock::new(steps_per_day); // Simulate 1 day

    let mut load = BaseLoad::new(
        0.8,           /* base_kw */
        0.7,           /* amp_kw */
        1.2,           /* phase_rad */
        0.05,          /* noise_std */
        steps_per_day, /* steps_per_day */
        42,            /* seed */
    );

    let baseload_device = load.device_type();
    let mut baseline_load = load.clone();
    let mut baseline = Vec::with_capacity(steps_per_day);
    for t in 0..steps_per_day {
        baseline.push(baseline_load.power_kw(&DeviceContext::new(t)));
    }
    let forecaster = NaiveForecast;
    let load_forecast = forecaster.forecast(&baseline, steps_per_day);
    let target_schedule = DayAheadSchedule::flat_target(&load_forecast);

    let mut pv = SolarPv::new(
        5.0,           /* kw_peak */
        steps_per_day, /* steps_per_day */
        6,             /* sunrise_idx (6 AM) */
        18,            /* sunset_idx (6 PM) */
        0.05,          /* noise_std */
        42,            /* seed */
    );

    let solar_device = pv.device_type();

    let mut battery = Battery::new(
        10.0,          /* capacity_kwh */
        0.5,           /* initial_soc */
        5.0,           /* max_charge_kw */
        5.0,           /* max_discharge_kw */
        0.95,          /* eta_c */
        0.95,          /* eta_d */
        steps_per_day, /* steps_per_day */
    );

    let battery_device = battery.device_type();
    let mut ev = EvCharger::new(
        7.2,           /* max_charge_kw */
        steps_per_day, /* steps_per_day */
        4.0,           /* demand_kwh_min */
        14.0,          /* demand_kwh_max */
        3,             /* dwell_steps_min */
        10,            /* dwell_steps_max */
        99,            /* seed */
    );
    let ev_device = ev.device_type();

    let mut feeder = Feeder::with_limits(
        "MainFeeder",
        5.0, /* max_import_kw */
        4.0, /* max_export_kw */
    );

    // Example external DR event: request 1.5kW reduction from hour 17 to 21.
    let dr_event = DemandResponseEvent::new(17, 21, 1.5);

    let controller = NaiveRtController;

    let mut telemetry = Vec::with_capacity(steps_per_day);
    let mut tracking_error_sq_sum = 0.0_f32;
    let mut tracking_error_count = 0_usize;
    let mut requested_curtailment_sum_kw = 0.0_f32;
    let mut achieved_curtailment_sum_kw = 0.0_f32;
    let mut feeder_peak_load_kw = 0.0_f32;

    clock.run(|t| {
        let context = DeviceContext::new(t);

        let base_demand_kw_raw = load.power_kw(&context);
        let forecast_kw = load_forecast[context.timestep];
        let target_kw = target_schedule[context.timestep];
        let solar_kw = pv.power_kw(&context);
        let ev_requested_kw = ev.requested_power_kw(&context);

        let dr_requested_kw = dr_event.requested_reduction_at_kw(t);
        let (base_demand_kw, ev_after_dr_kw, dr_achieved_kw) = controller.apply_demand_response_kw(
            base_demand_kw_raw,
            ev_requested_kw,
            dr_requested_kw,
        );

        let net_fixed_kw = base_demand_kw - solar_kw;
        let ev_capped_kw = controller.capped_flexible_load_kw(
            net_fixed_kw,
            ev_after_dr_kw,
            feeder.max_import_kw(),
            battery.max_discharge_kw,
        );
        let ev_context = DeviceContext::with_setpoint(context.timestep, ev_capped_kw);
        let ev_kw = ev.power_kw(&ev_context);

        let net_without_battery = net_fixed_kw + ev_kw;
        let battery_setpoint_kw = controller.constrained_battery_setpoint_kw(
            net_without_battery,
            target_kw,
            feeder.max_import_kw(),
            feeder.max_export_kw(),
            battery.max_charge_kw,
            battery.max_discharge_kw,
        );
        let battery_context = DeviceContext::with_setpoint(context.timestep, battery_setpoint_kw);

        let battery_kw = battery.power_kw(&battery_context);
        feeder.reset();
        feeder.add_net_kw(base_demand_kw);
        feeder.add_net_kw(ev_kw);
        feeder.add_net_kw(-solar_kw);
        feeder.add_net_kw(-battery_kw);
        let feeder_kw = feeder.net_kw();
        let tracking_error_kw = feeder_kw - target_kw;
        let feeder_name = feeder.name();

        tracking_error_sq_sum += tracking_error_kw * tracking_error_kw;
        tracking_error_count += 1;
        requested_curtailment_sum_kw += dr_requested_kw;
        achieved_curtailment_sum_kw += dr_achieved_kw;
        feeder_peak_load_kw = feeder_peak_load_kw.max(feeder_kw);

        let row = TelemetryRow {
            timestep: t,
            time_hr: t as f32 * dt_hr,
            target_kw,
            feeder_kw,
            tracking_error_kw,
            baseload_kw: base_demand_kw,
            solar_kw,
            ev_requested_kw,
            ev_dispatched_kw: ev_kw,
            battery_kw,
            battery_soc: battery.soc,
            dr_requested_kw,
            dr_achieved_kw,
            limit_ok: feeder.within_limits(),
        };
        telemetry.push(row);

        let soc = battery.soc * 100.0;
        if print_readable_log {
            println!(
                "Time (Hr) {t}: {baseload_device}={base_demand_kw:.2} kW, \
                RawBase={base_demand_kw_raw:.2} kW, \
                Forecast={forecast_kw:.2} kW, \
                Target={target_kw:.2} kW, \
                {solar_device}={solar_kw:.2} kW, \
                {ev_device}={ev_kw:.2} kW (Req={ev_requested_kw:.2}, DR={ev_after_dr_kw:.2}, Cap={ev_capped_kw:.2}), \
                {battery_device}={battery_kw:.2} kW (SoC={soc:.1}%), \
                {feeder_name}={feeder_kw:.2} kW, \
                Error={tracking_error_kw:.2} kW, \
                DR(req={dr_requested_kw:.2}, done={dr_achieved_kw:.2}), \
                LimitOK={}",
                feeder.within_limits()
            );
        }
    });

    let rmse_tracking_kw = if tracking_error_count > 0 {
        (tracking_error_sq_sum / tracking_error_count as f32).sqrt()
    } else {
        0.0
    };

    let curtailment_pct = if requested_curtailment_sum_kw > 0.0 {
        100.0 * achieved_curtailment_sum_kw / requested_curtailment_sum_kw
    } else {
        0.0
    };

    SimulationResult {
        telemetry,
        kpis: SimulationKpis {
            rmse_tracking_kw,
            curtailment_pct,
            feeder_peak_load_kw,
        },
    }
}

fn main() {
    let opts = match parse_args() {
        Ok(opts) => opts,
        Err(err) => {
            eprintln!("Error: {err}");
            print_usage();
            std::process::exit(2);
        }
    };

    let result = run_demo_simulation(true);
    if let Some(path) = opts.telemetry_out.as_deref() {
        if let Err(err) = write_telemetry_to_path(path, &result.telemetry) {
            eprintln!(
                "Error: failed to write telemetry CSV to {}: {err}",
                path.display()
            );
            std::process::exit(1);
        }
    }

    println!("\n--- KPI Report ---");
    println!(
        "RMSE tracking error: {:.3} kW",
        result.kpis.rmse_tracking_kw
    );
    println!("Curtailment achieved: {:.1}%", result.kpis.curtailment_pct);
    println!(
        "Feeder peak load: {:.2} kW",
        result.kpis.feeder_peak_load_kw
    );
}

#[cfg(test)]
mod tests {
    use super::{TELEMETRY_SCHEMA_V1_HEADER, run_demo_simulation, write_telemetry_csv};

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
