use std::env;
use std::path::PathBuf;

pub struct CliOptions {
    pub scenario: Option<PathBuf>,
    pub preset: Option<String>,
    pub telemetry_out: Option<PathBuf>,
    pub api_bind: Option<String>,
}

pub fn parse_args() -> Result<CliOptions, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    parse_args_from(args)
}

fn parse_args_from(args: Vec<String>) -> Result<CliOptions, String> {
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        print_usage();
        std::process::exit(0);
    }
    parse_options(&args)
}

fn parse_options(args: &[String]) -> Result<CliOptions, String> {
    let mut i = 0usize;
    let mut scenario = None;
    let mut preset = None;
    let mut telemetry_out = None;
    let mut api_bind = None;

    while i < args.len() {
        match args[i].as_str() {
            "--scenario" => {
                i += 1;
                let path = args.get(i).ok_or_else(|| {
                    "missing value for --scenario (expected a JSON file path)".to_string()
                })?;
                if scenario.replace(PathBuf::from(path)).is_some() {
                    return Err("--scenario provided more than once".to_string());
                }
            }
            "--preset" => {
                i += 1;
                let name = args.get(i).ok_or_else(|| {
                    "missing value for --preset (expected a preset name)".to_string()
                })?;
                if preset.replace(name.clone()).is_some() {
                    return Err("--preset provided more than once".to_string());
                }
            }
            "--telemetry-out" => {
                i += 1;
                let path = args.next_or_err(
                    i,
                    "missing value for --telemetry-out (expected a file path)",
                )?;
                if telemetry_out.replace(PathBuf::from(path)).is_some() {
                    return Err("--telemetry-out provided more than once".to_string());
                }
            }
            "--api-bind" => {
                i += 1;
                let addr =
                    args.next_or_err(i, "missing value for --api-bind (expected host:port)")?;
                if api_bind.replace(addr.to_string()).is_some() {
                    return Err("--api-bind provided more than once".to_string());
                }
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }

    if scenario.is_some() && preset.is_some() {
        return Err(
            "arguments `--scenario` and `--preset` are mutually exclusive; choose one source"
                .to_string(),
        );
    }

    if scenario.is_none() && preset.is_none() {
        preset = Some("demo".to_string());
    }

    Ok(CliOptions {
        scenario,
        preset,
        telemetry_out,
        api_bind,
    })
}

trait SliceArgExt {
    fn next_or_err(&self, index: usize, err: &str) -> Result<&str, String>;
}

impl SliceArgExt for [String] {
    fn next_or_err(&self, index: usize, err: &str) -> Result<&str, String> {
        self.get(index)
            .map(String::as_str)
            .ok_or_else(|| err.to_string())
    }
}

pub fn print_usage() {
    eprintln!("Usage:");
    eprintln!(
        "  cargo run --release -- [--scenario <path> | --preset <name>] [--telemetry-out <path>] [--api-bind <host:port>]"
    );
}

#[cfg(test)]
mod tests {
    use super::parse_args_from;

    #[test]
    fn supports_scenario_cli() {
        let opts = parse_args_from(vec!["--scenario".to_string(), "scenario.json".to_string()])
            .expect("parse should succeed");
        assert_eq!(
            opts.scenario.as_deref().and_then(|p| p.to_str()),
            Some("scenario.json")
        );
        assert!(opts.preset.is_none());
    }

    #[test]
    fn supports_preset_cli() {
        let opts = parse_args_from(vec!["--preset".to_string(), "demo".to_string()])
            .expect("parse should succeed");
        assert_eq!(opts.preset.as_deref(), Some("demo"));
        assert!(opts.scenario.is_none());
    }

    #[test]
    fn supports_api_bind_cli() {
        let opts = parse_args_from(vec![
            "--preset".to_string(),
            "demo".to_string(),
            "--api-bind".to_string(),
            "127.0.0.1:8080".to_string(),
        ])
        .expect("parse should succeed");
        assert_eq!(opts.api_bind.as_deref(), Some("127.0.0.1:8080"));
    }
}
