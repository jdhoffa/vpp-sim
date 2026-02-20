mod api;
mod cli;
mod devices;
mod forecast;
mod reporting;
mod runner;
mod scenario;
mod sim;
mod telemetry;

use api::run_http_server;
use cli::{parse_args, print_usage};
use reporting::print_kpi_report;
use runner::run_scenario;
use scenario::ScenarioConfig;
use telemetry::write_telemetry_to_path;

fn main() {
    let opts = match parse_args() {
        Ok(opts) => opts,
        Err(err) => {
            eprintln!("Error: {err}");
            print_usage();
            std::process::exit(2);
        }
    };

    let scenario = if let Some(path) = opts.scenario.as_deref() {
        match ScenarioConfig::from_json_path(path) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("Error: {err}");
                std::process::exit(2);
            }
        }
    } else if let Some(preset) = opts.preset.as_deref() {
        match ScenarioConfig::from_preset(preset) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("Error: {err}");
                std::process::exit(2);
            }
        }
    } else {
        ScenarioConfig::default()
    };

    let result = run_scenario(&scenario, true);
    if let Some(path) = opts.telemetry_out.as_deref() {
        if let Err(err) = write_telemetry_to_path(path, &result.telemetry) {
            eprintln!(
                "Error: failed to write telemetry CSV to {}: {err}",
                path.display()
            );
            std::process::exit(1);
        }
    }

    print_kpi_report(&result.kpis);

    if let Some(bind_addr) = opts.api_bind.as_deref() {
        if let Err(err) = run_http_server(bind_addr, result.telemetry.clone()) {
            eprintln!("Error: failed to start HTTP API on {bind_addr}: {err}");
            std::process::exit(1);
        }
    }
}
