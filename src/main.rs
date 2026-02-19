mod cli;
mod devices;
mod forecast;
mod reporting;
mod runner;
mod sim;
mod telemetry;

use cli::{parse_args, print_usage};
use reporting::print_kpi_report;
use runner::run_demo;
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

    let result = run_demo(true);
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
}
