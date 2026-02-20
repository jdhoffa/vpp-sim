use std::process::Command;

#[derive(Debug)]
struct Kpis {
    rmse_tracking_kw: f64,
    curtailment_pct: f64,
}

#[test]
fn scenario_presets_run_via_cli_and_produce_distinct_dynamics() {
    let baseline = run_and_parse_kpis("scenarios/baseline.toml");
    let high_solar = run_and_parse_kpis("scenarios/high_solar.toml");
    let dr_event = run_and_parse_kpis("scenarios/dr_event.toml");

    assert!(
        (baseline.rmse_tracking_kw - high_solar.rmse_tracking_kw).abs() > 1.0,
        "expected baseline and high_solar RMSE to differ: baseline={:.3}, high_solar={:.3}",
        baseline.rmse_tracking_kw,
        high_solar.rmse_tracking_kw
    );

    assert!(
        (baseline.curtailment_pct - dr_event.curtailment_pct).abs() > 1.0,
        "expected baseline and dr_event curtailment to differ: baseline={:.3}, dr_event={:.3}",
        baseline.curtailment_pct,
        dr_event.curtailment_pct
    );

    assert!(
        (high_solar.rmse_tracking_kw - dr_event.rmse_tracking_kw).abs() > 0.01,
        "expected high_solar and dr_event RMSE to differ: high_solar={:.3}, dr_event={:.3}",
        high_solar.rmse_tracking_kw,
        dr_event.rmse_tracking_kw
    );

    assert!(
        (high_solar.curtailment_pct - dr_event.curtailment_pct).abs() > 1.0,
        "expected high_solar and dr_event curtailment to differ: high_solar={:.3}, dr_event={:.3}",
        high_solar.curtailment_pct,
        dr_event.curtailment_pct
    );
}

fn run_and_parse_kpis(path: &str) -> Kpis {
    let output = Command::new(env!("CARGO_BIN_EXE_vpp-sim"))
        .args(["--scenario", path])
        .output()
        .expect("vpp-sim process should run");

    assert!(
        output.status.success(),
        "scenario run failed for {path}: stderr={} ",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid UTF-8");
    parse_kpis(&stdout)
}

fn parse_kpis(stdout: &str) -> Kpis {
    let rmse_tracking_kw = parse_metric(stdout, "RMSE tracking error:", "kW");
    let curtailment_pct = parse_metric(stdout, "Curtailment achieved:", "%");

    Kpis {
        rmse_tracking_kw,
        curtailment_pct,
    }
}

fn parse_metric(stdout: &str, label: &str, unit: &str) -> f64 {
    let line = stdout
        .lines()
        .find(|line| line.trim_start().starts_with(label))
        .unwrap_or_else(|| panic!("missing KPI line `{label}` in output: {stdout}"));

    let raw = line
        .split_once(':')
        .map(|(_, right)| right.trim())
        .unwrap_or_else(|| panic!("invalid KPI format for line `{line}`"));

    let numeric = raw.strip_suffix(unit).unwrap_or(raw).trim();
    numeric
        .parse::<f64>()
        .unwrap_or_else(|_| panic!("failed parsing `{numeric}` from KPI line `{line}`"))
}
