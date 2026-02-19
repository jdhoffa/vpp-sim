use std::env;
use std::path::PathBuf;

pub struct CliOptions {
    pub telemetry_out: Option<PathBuf>,
}

pub fn parse_args() -> Result<CliOptions, String> {
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

pub fn print_usage() {
    eprintln!("Usage: cargo run --release -- [--telemetry-out <path>]");
}
