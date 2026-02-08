//! VPP simulator entry point — CLI wiring and config-driven engine construction.

use std::path::Path;
use std::process;

use vpp_sim::config::ScenarioConfig;

/// Seed offset for the EV charger RNG to avoid correlation with other devices.
const EV_SEED_OFFSET: u64 = 57;
use vpp_sim::devices::{
    BaseLoad, Battery, Device, DeviceContext, EvCharger, Solar, SolarPv, SolarPvAr1,
};
use vpp_sim::forecast::NaiveForecast;
use vpp_sim::io::export::export_csv;
use vpp_sim::sim::controller::{GreedyController, NaiveRtController};
use vpp_sim::sim::engine::Engine;
use vpp_sim::sim::event::DemandResponseEvent;
use vpp_sim::sim::feeder::Feeder;
use vpp_sim::sim::kpi::KpiReport;
use vpp_sim::sim::schedule::DayAheadSchedule;
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

/// Builds shared scenario components (devices, forecast, schedule, DR event).
///
/// Returns `(sim_config, load, pv, battery, ev, feeder, load_forecast, target_schedule, dr_event)`.
#[expect(clippy::type_complexity)]
fn build_scenario(
    cfg: &ScenarioConfig,
) -> (
    SimConfig,
    BaseLoad,
    Solar,
    Battery,
    EvCharger,
    Feeder,
    Vec<f32>,
    Vec<f32>,
    DemandResponseEvent,
) {
    let s = &cfg.simulation;
    let mut sim_config = SimConfig::new(s.steps_per_day, s.days, s.seed);
    sim_config.imbalance_price_per_kwh = s.imbalance_price_per_kwh;

    // Build baseline forecast from a throwaway load instance
    let bl = &cfg.baseload;
    let mut baseline_load = BaseLoad::new(
        bl.base_kw,
        bl.amp_kw,
        bl.phase_rad,
        bl.noise_std,
        &sim_config,
        s.seed,
    );
    let mut baseline = Vec::with_capacity(sim_config.steps_per_day);
    for t in 0..sim_config.steps_per_day {
        baseline.push(baseline_load.power_kw(&DeviceContext::new(t)));
    }

    let load = BaseLoad::new(
        bl.base_kw,
        bl.amp_kw,
        bl.phase_rad,
        bl.noise_std,
        &sim_config,
        s.seed,
    );

    let sol = &cfg.solar;
    let pv: Solar = match sol.model.as_str() {
        "ar1" => Solar::Ar1(SolarPvAr1::new(
            sol.kw_peak,
            sol.sunrise_idx,
            sol.sunset_idx,
            sol.alpha,
            sol.cloud_noise_std,
            &sim_config,
            s.seed,
        )),
        _ => Solar::Simple(SolarPv::new(
            sol.kw_peak,
            sol.sunrise_idx,
            sol.sunset_idx,
            sol.noise_std,
            &sim_config,
            s.seed,
        )),
    };

    let bat = &cfg.battery;
    let battery = Battery::new(
        bat.capacity_kwh,
        bat.initial_soc,
        bat.max_charge_kw,
        bat.max_discharge_kw,
        bat.eta_charge,
        bat.eta_discharge,
        &sim_config,
    );

    let e = &cfg.ev;
    let ev = EvCharger::new(
        e.max_charge_kw,
        e.demand_kwh_min,
        e.demand_kwh_max,
        e.dwell_steps_min,
        e.dwell_steps_max,
        &sim_config,
        s.seed.wrapping_add(EV_SEED_OFFSET),
    );

    let f = &cfg.feeder;
    let feeder = Feeder::with_limits("MainFeeder", f.max_import_kw, f.max_export_kw);

    let forecaster = NaiveForecast;
    let load_forecast = forecaster.forecast(&baseline, sim_config.steps_per_day);
    let target_schedule = DayAheadSchedule::flat_target(&load_forecast);

    let dr = &cfg.dr_event;
    let dr_event = DemandResponseEvent::new(dr.start_step, dr.end_step, dr.requested_reduction_kw);

    (
        sim_config,
        load,
        pv,
        battery,
        ev,
        feeder,
        load_forecast,
        target_schedule,
        dr_event,
    )
}

/// Runs the simulation with the configured controller and returns config, results, and KPI.
fn run_simulation(
    cfg: &ScenarioConfig,
) -> (SimConfig, Vec<vpp_sim::sim::types::StepResult>, KpiReport) {
    let (sim_config, load, pv, battery, ev, feeder, load_forecast, target_schedule, dr_event) =
        build_scenario(cfg);

    let bat_cap = battery.capacity_kwh;
    let dt = sim_config.dt_hours;

    if cfg.simulation.controller == "greedy" {
        let controller = GreedyController::new(
            &load_forecast,
            &target_schedule,
            cfg.battery.capacity_kwh,
            cfg.battery.max_charge_kw,
            cfg.battery.max_discharge_kw,
            cfg.battery.initial_soc,
            cfg.battery.eta_charge,
            cfg.battery.eta_discharge,
            sim_config.dt_hours,
            cfg.solar.kw_peak,
            cfg.solar.sunrise_idx,
            cfg.solar.sunset_idx,
        );
        let mut engine = Engine::new(
            sim_config.clone(),
            load,
            pv,
            battery,
            ev,
            feeder,
            controller,
            load_forecast,
            target_schedule,
            dr_event,
        );
        let results = engine.run();
        let kpi = KpiReport::from_results(&results, dt, bat_cap);
        (sim_config, results, kpi)
    } else {
        let mut engine = Engine::new(
            sim_config.clone(),
            load,
            pv,
            battery,
            ev,
            feeder,
            NaiveRtController,
            load_forecast,
            target_schedule,
            dr_event,
        );
        let results = engine.run();
        let kpi = KpiReport::from_results(&results, dt, bat_cap);
        (sim_config, results, kpi)
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

    // Build and run
    #[expect(unused_variables)]
    let (sim_config, results, kpi) = run_simulation(&scenario);

    // Print per-step results
    for r in &results {
        println!("{r}");
    }

    // Print KPI report
    println!("\n{kpi}");

    // Export CSV if requested
    if let Some(ref path) = cli.telemetry_out {
        if let Err(e) = export_csv(&results, Path::new(path)) {
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
