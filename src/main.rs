//! VPP simulator entry point — CLI wiring and config-driven engine construction.

use std::path::Path;
use std::process;

use vpp_sim::config::ScenarioConfig;
use vpp_sim::sim::controller::{GreedyController, NaiveRtController};
use vpp_sim::sim::engine::Engine;
use vpp_sim::sim::kpi::KpiReport;
use vpp_sim::sim::types::SimConfig;

/// Parsed CLI arguments.
struct CliArgs {
    scenario_path: Option<String>,
    preset: Option<String>,
    seed_override: Option<u64>,
    telemetry_out: Option<String>,
    #[cfg(feature = "api")]
    serve: bool,
    #[cfg(feature = "api")]
    port: u16,
    #[cfg(feature = "tui")]
    tui: bool,
}

fn print_help() {
    eprintln!("vpp-sim — Neighborhood-scale Virtual Power Plant simulator");
    eprintln!();
    eprintln!("Usage: vpp-sim [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --scenario <path>        Load scenario from TOML config file");
    eprintln!("  --preset <name>          Use a built-in preset (baseline)");
    eprintln!("  --seed <u64>             Override random seed");
    eprintln!("  --telemetry-out <path>   Export step results to CSV");
    #[cfg(feature = "api")]
    {
        eprintln!("  --serve                  Start REST API server after simulation");
        eprintln!("  --port <u16>             API server port (default: 3000)");
    }
    #[cfg(feature = "tui")]
    eprintln!("  --tui                    Launch live terminal UI");
    eprintln!("  --help                   Show this help message");
    eprintln!();
    eprintln!("If no --scenario or --preset is given, the baseline preset is used.");
}

fn parse_args() -> CliArgs {
    let args: Vec<String> = std::env::args().collect();
    let mut cli = CliArgs {
        scenario_path: None,
        preset: None,
        seed_override: None,
        telemetry_out: None,
        #[cfg(feature = "api")]
        serve: false,
        #[cfg(feature = "api")]
        port: 3000,
        #[cfg(feature = "tui")]
        tui: false,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help();
                process::exit(0);
            }
            "--scenario" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --scenario requires a path argument");
                    process::exit(1);
                }
                cli.scenario_path = Some(args[i].clone());
            }
            "--preset" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --preset requires a name argument");
                    process::exit(1);
                }
                cli.preset = Some(args[i].clone());
            }
            "--seed" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --seed requires a u64 argument");
                    process::exit(1);
                }
                if let Ok(s) = args[i].parse::<u64>() {
                    cli.seed_override = Some(s);
                } else {
                    eprintln!("error: --seed value \"{}\" is not a valid u64", args[i]);
                    process::exit(1);
                }
            }
            "--telemetry-out" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --telemetry-out requires a path argument");
                    process::exit(1);
                }
                cli.telemetry_out = Some(args[i].clone());
            }
            #[cfg(feature = "api")]
            "--serve" => {
                cli.serve = true;
            }
            #[cfg(feature = "api")]
            "--port" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --port requires a u16 argument");
                    process::exit(1);
                }
                if let Ok(p) = args[i].parse::<u16>() {
                    cli.port = p;
                } else {
                    eprintln!("error: --port value \"{}\" is not a valid u16", args[i]);
                    process::exit(1);
                }
            }
            #[cfg(feature = "tui")]
            "--tui" => {
                cli.tui = true;
            }
            other => {
                eprintln!("error: unknown argument \"{other}\"");
                print_help();
                process::exit(1);
            }
        }
        i += 1;
    }

    cli
}

/// Runs the simulation with the configured controller and returns config, results, and KPI.
fn run_simulation(
    cfg: &ScenarioConfig,
) -> (SimConfig, Vec<vpp_sim::sim::types::StepResult>, KpiReport) {
    let c = cfg.build();
    let bat_cap = c.battery.capacity_kwh;
    let dt = c.sim_config.dt_hours;

    if cfg.simulation.controller == "greedy" {
        let controller = GreedyController::new(
            &c.load_forecast,
            &c.target_schedule,
            cfg.battery.capacity_kwh,
            cfg.battery.max_charge_kw,
            cfg.battery.max_discharge_kw,
            cfg.battery.initial_soc,
            cfg.battery.eta_charge,
            cfg.battery.eta_discharge,
            c.sim_config.dt_hours,
            cfg.solar.kw_peak,
            cfg.solar.sunrise_idx,
            cfg.solar.sunset_idx,
        );
        let mut engine = Engine::new(
            c.sim_config,
            c.load,
            c.pv,
            c.battery,
            c.ev,
            c.feeder,
            controller,
            c.load_forecast,
            c.target_schedule,
            c.dr_event,
        );
        let results = engine.run();
        let kpi = KpiReport::from_results(&results, dt, bat_cap);
        (engine.config().clone(), results, kpi)
    } else {
        let mut engine = Engine::new(
            c.sim_config,
            c.load,
            c.pv,
            c.battery,
            c.ev,
            c.feeder,
            NaiveRtController,
            c.load_forecast,
            c.target_schedule,
            c.dr_event,
        );
        let results = engine.run();
        let kpi = KpiReport::from_results(&results, dt, bat_cap);
        (engine.config().clone(), results, kpi)
    }
}

fn main() {
    let cli = parse_args();

    // Load config: --scenario takes priority, then --preset, then baseline default
    let mut scenario = if let Some(ref path) = cli.scenario_path {
        match ScenarioConfig::from_toml_file(Path::new(path)) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("{e}");
                process::exit(1);
            }
        }
    } else if let Some(ref name) = cli.preset {
        match ScenarioConfig::from_preset(name) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("{e}");
                process::exit(1);
            }
        }
    } else {
        ScenarioConfig::baseline()
    };

    // Apply seed override
    if let Some(seed) = cli.seed_override {
        scenario.simulation.seed = seed;
    }

    // Validate
    let errors = scenario.validate();
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("{e}");
        }
        process::exit(1);
    }

    // Launch TUI if requested
    #[cfg(feature = "tui")]
    if cli.tui {
        let preset = cli.preset.as_deref().unwrap_or("baseline");
        vpp_sim::tui::run(preset);
        return;
    }

    // Build and run
    #[cfg_attr(not(feature = "api"), expect(unused_variables))]
    let (sim_config, results, kpi) = run_simulation(&scenario);

    // Print per-step results
    for r in &results {
        println!("{r}");
    }

    // Print KPI report
    println!("\n{kpi}");

    // Export CSV if requested
    if let Some(ref path) = cli.telemetry_out {
        if let Err(e) = vpp_sim::io::export::export_csv(&results, Path::new(path)) {
            eprintln!("error: failed to write CSV: {e}");
            process::exit(1);
        }
        eprintln!("Telemetry written to {path}");
    }

    // Start API server if requested
    #[cfg(feature = "api")]
    if cli.serve {
        use std::net::SocketAddr;
        use std::sync::Arc;

        let state = Arc::new(vpp_sim::api::AppState {
            config: sim_config,
            kpi,
            results,
        });
        let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
        let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| {
            eprintln!("error: failed to create tokio runtime: {e}");
            process::exit(1);
        });
        rt.block_on(vpp_sim::api::serve(state, addr));
    }
}
