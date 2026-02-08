//! TOML-based scenario configuration and preset definitions.

use std::fmt;
use std::fs;
use std::path::Path;

use serde::Deserialize;

/// Top-level scenario configuration parsed from TOML.
///
/// All fields have defaults matching the baseline scenario. Load from
/// TOML with [`ScenarioConfig::from_toml_file`] or use
/// [`ScenarioConfig::baseline`] for the built-in default.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioConfig {
    /// Simulation timing and global parameters.
    #[serde(default)]
    pub simulation: SimulationConfig,
    /// Baseload device parameters.
    #[serde(default)]
    pub baseload: BaseloadConfig,
    /// Solar PV device parameters.
    #[serde(default)]
    pub solar: SolarConfig,
    /// Battery storage parameters.
    #[serde(default)]
    pub battery: BatteryConfig,
    /// EV charger parameters.
    #[serde(default)]
    pub ev: EvConfig,
    /// Feeder import/export limits.
    #[serde(default)]
    pub feeder: FeederConfig,
    /// Demand response event parameters.
    #[serde(default)]
    pub dr_event: DrEventConfig,
}

/// Simulation timing and global parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SimulationConfig {
    /// Number of timesteps per simulated day (must be > 0).
    pub steps_per_day: usize,
    /// Number of days to simulate (must be > 0).
    pub days: usize,
    /// Master random seed.
    pub seed: u64,
    /// Imbalance settlement price per kWh.
    pub imbalance_price_per_kwh: f32,
    /// Controller type: `"naive"` or `"greedy"`.
    pub controller: String,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            steps_per_day: 24,
            days: 1,
            seed: 42,
            imbalance_price_per_kwh: 0.10,
            controller: "naive".to_string(),
        }
    }
}

/// Baseload device parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BaseloadConfig {
    /// Baseline consumption (kW).
    pub base_kw: f32,
    /// Sinusoidal amplitude (kW).
    pub amp_kw: f32,
    /// Phase offset (radians).
    pub phase_rad: f32,
    /// Gaussian noise standard deviation (kW).
    pub noise_std: f32,
}

impl Default for BaseloadConfig {
    fn default() -> Self {
        Self {
            base_kw: 0.8,
            amp_kw: 0.7,
            phase_rad: 1.2,
            noise_std: 0.05,
        }
    }
}

/// Solar PV device parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SolarConfig {
    /// Solar model: `"simple"` (independent noise) or `"ar1"` (AR(1) cloud).
    pub model: String,
    /// Peak generation (kW).
    pub kw_peak: f32,
    /// Sunrise timestep index (inclusive).
    pub sunrise_idx: usize,
    /// Sunset timestep index (exclusive).
    pub sunset_idx: usize,
    /// Noise standard deviation for simple model.
    pub noise_std: f32,
    /// AR(1) correlation coefficient for ar1 model (0.0-1.0).
    pub alpha: f32,
    /// AR(1) innovation noise standard deviation for ar1 model.
    pub cloud_noise_std: f32,
}

impl Default for SolarConfig {
    fn default() -> Self {
        Self {
            model: "simple".to_string(),
            kw_peak: 5.0,
            sunrise_idx: 6,
            sunset_idx: 18,
            noise_std: 0.05,
            alpha: 0.9,
            cloud_noise_std: 0.2,
        }
    }
}

/// Battery storage parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BatteryConfig {
    /// Total energy capacity (kWh).
    pub capacity_kwh: f32,
    /// Initial state of charge (0.0–1.0).
    pub initial_soc: f32,
    /// Maximum charging power (kW).
    pub max_charge_kw: f32,
    /// Maximum discharging power (kW).
    pub max_discharge_kw: f32,
    /// Charge efficiency (0.0–1.0).
    pub eta_charge: f32,
    /// Discharge efficiency (0.0–1.0).
    pub eta_discharge: f32,
}

impl Default for BatteryConfig {
    fn default() -> Self {
        Self {
            capacity_kwh: 10.0,
            initial_soc: 0.5,
            max_charge_kw: 5.0,
            max_discharge_kw: 5.0,
            eta_charge: 0.95,
            eta_discharge: 0.95,
        }
    }
}

/// EV charger parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EvConfig {
    /// Maximum charging power (kW).
    pub max_charge_kw: f32,
    /// Minimum daily energy demand (kWh).
    pub demand_kwh_min: f32,
    /// Maximum daily energy demand (kWh).
    pub demand_kwh_max: f32,
    /// Minimum dwell duration (timesteps).
    pub dwell_steps_min: usize,
    /// Maximum dwell duration (timesteps).
    pub dwell_steps_max: usize,
}

impl Default for EvConfig {
    fn default() -> Self {
        Self {
            max_charge_kw: 7.2,
            demand_kwh_min: 4.0,
            demand_kwh_max: 14.0,
            dwell_steps_min: 3,
            dwell_steps_max: 10,
        }
    }
}

/// Feeder import/export limits.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FeederConfig {
    /// Maximum import power (kW).
    pub max_import_kw: f32,
    /// Maximum export power (kW, positive magnitude).
    pub max_export_kw: f32,
}

impl Default for FeederConfig {
    fn default() -> Self {
        Self {
            max_import_kw: 5.0,
            max_export_kw: 4.0,
        }
    }
}

/// Demand response event parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DrEventConfig {
    /// Start timestep (inclusive).
    pub start_step: usize,
    /// End timestep (exclusive).
    pub end_step: usize,
    /// Requested reduction (kW).
    pub requested_reduction_kw: f32,
}

impl Default for DrEventConfig {
    fn default() -> Self {
        Self {
            start_step: 17,
            end_step: 21,
            requested_reduction_kw: 1.5,
        }
    }
}

/// Configuration error with field path and constraint description.
#[derive(Debug)]
pub struct ConfigError {
    /// Dotted field path (e.g., `"simulation.steps_per_day"`).
    pub field: String,
    /// Human-readable constraint description.
    pub message: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "config error: {} — {}", self.field, self.message)
    }
}

impl ScenarioConfig {
    /// Returns the baseline scenario (same parameters as the original hardcoded defaults).
    pub fn baseline() -> Self {
        Self {
            simulation: SimulationConfig::default(),
            baseload: BaseloadConfig::default(),
            solar: SolarConfig::default(),
            battery: BatteryConfig::default(),
            ev: EvConfig::default(),
            feeder: FeederConfig::default(),
            dr_event: DrEventConfig::default(),
        }
    }

    /// Returns the high-solar preset: large PV array with AR(1) cloud model.
    pub fn high_solar() -> Self {
        Self {
            simulation: SimulationConfig::default(),
            baseload: BaseloadConfig {
                base_kw: 0.6,
                amp_kw: 0.4,
                noise_std: 0.03,
                ..BaseloadConfig::default()
            },
            solar: SolarConfig {
                model: "ar1".to_string(),
                kw_peak: 12.0,
                sunrise_idx: 5,
                sunset_idx: 19,
                alpha: 0.9,
                cloud_noise_std: 0.25,
                ..SolarConfig::default()
            },
            battery: BatteryConfig {
                capacity_kwh: 15.0,
                initial_soc: 0.3,
                max_charge_kw: 7.0,
                max_discharge_kw: 7.0,
                ..BatteryConfig::default()
            },
            ev: EvConfig::default(),
            feeder: FeederConfig {
                max_export_kw: 10.0,
                ..FeederConfig::default()
            },
            dr_event: DrEventConfig {
                requested_reduction_kw: 1.0,
                ..DrEventConfig::default()
            },
        }
    }

    /// Returns the DR-stress preset: aggressive DR, tight feeder limits.
    pub fn dr_stress() -> Self {
        Self {
            simulation: SimulationConfig {
                imbalance_price_per_kwh: 0.25,
                ..SimulationConfig::default()
            },
            baseload: BaseloadConfig {
                base_kw: 1.2,
                amp_kw: 0.8,
                ..BaseloadConfig::default()
            },
            solar: SolarConfig {
                kw_peak: 4.0,
                ..SolarConfig::default()
            },
            battery: BatteryConfig {
                capacity_kwh: 8.0,
                max_charge_kw: 4.0,
                max_discharge_kw: 4.0,
                eta_charge: 0.90,
                eta_discharge: 0.90,
                ..BatteryConfig::default()
            },
            ev: EvConfig {
                demand_kwh_min: 6.0,
                demand_kwh_max: 18.0,
                dwell_steps_min: 2,
                dwell_steps_max: 8,
                ..EvConfig::default()
            },
            feeder: FeederConfig {
                max_import_kw: 3.0,
                max_export_kw: 2.0,
            },
            dr_event: DrEventConfig {
                start_step: 14,
                end_step: 22,
                requested_reduction_kw: 3.0,
            },
        }
    }

    /// Available preset names.
    pub const PRESETS: &[&str] = &["baseline", "high_solar", "dr_stress"];

    /// Loads a scenario from a named preset.
    ///
    /// # Errors
    ///
    /// Returns a `ConfigError` if the preset name is unknown.
    pub fn from_preset(name: &str) -> Result<Self, ConfigError> {
        match name {
            "baseline" => Ok(Self::baseline()),
            "high_solar" => Ok(Self::high_solar()),
            "dr_stress" => Ok(Self::dr_stress()),
            _ => Err(ConfigError {
                field: "preset".to_string(),
                message: format!(
                    "unknown preset \"{name}\", available: {}",
                    Self::PRESETS.join(", ")
                ),
            }),
        }
    }

    /// Parses a scenario from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns a `ConfigError` if the file cannot be read or the TOML is invalid.
    pub fn from_toml_file(path: &Path) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path).map_err(|e| ConfigError {
            field: "scenario".to_string(),
            message: format!("cannot read \"{}\": {e}", path.display()),
        })?;
        Self::from_toml_str(&content)
    }

    /// Parses a scenario from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns a `ConfigError` if the TOML is invalid or contains unknown fields.
    pub fn from_toml_str(s: &str) -> Result<Self, ConfigError> {
        toml::from_str(s).map_err(|e| ConfigError {
            field: "toml".to_string(),
            message: e.to_string(),
        })
    }

    /// Validates all fields and returns a list of errors.
    ///
    /// Returns an empty vector if configuration is valid.
    pub fn validate(&self) -> Vec<ConfigError> {
        let mut errors = Vec::new();
        let s = &self.simulation;

        if s.steps_per_day == 0 {
            errors.push(ConfigError {
                field: "simulation.steps_per_day".into(),
                message: "must be > 0".into(),
            });
        }
        if s.days == 0 {
            errors.push(ConfigError {
                field: "simulation.days".into(),
                message: "must be > 0".into(),
            });
        }
        if s.controller != "naive" && s.controller != "greedy" {
            errors.push(ConfigError {
                field: "simulation.controller".into(),
                message: format!("must be \"naive\" or \"greedy\", got \"{}\"", s.controller),
            });
        }

        let sol = &self.solar;
        if sol.model != "simple" && sol.model != "ar1" {
            errors.push(ConfigError {
                field: "solar.model".into(),
                message: format!("must be \"simple\" or \"ar1\", got \"{}\"", sol.model),
            });
        }
        if sol.sunrise_idx >= sol.sunset_idx {
            errors.push(ConfigError {
                field: "solar.sunrise_idx".into(),
                message: "must be < solar.sunset_idx".into(),
            });
        }
        if s.steps_per_day > 0 && sol.sunset_idx > s.steps_per_day {
            errors.push(ConfigError {
                field: "solar.sunset_idx".into(),
                message: "must be <= simulation.steps_per_day".into(),
            });
        }

        let bat = &self.battery;
        if bat.capacity_kwh <= 0.0 {
            errors.push(ConfigError {
                field: "battery.capacity_kwh".into(),
                message: "must be > 0".into(),
            });
        }
        if !(0.0..=1.0).contains(&bat.initial_soc) {
            errors.push(ConfigError {
                field: "battery.initial_soc".into(),
                message: "must be in [0.0, 1.0]".into(),
            });
        }

        let ev = &self.ev;
        if ev.dwell_steps_min > ev.dwell_steps_max {
            errors.push(ConfigError {
                field: "ev.dwell_steps_min".into(),
                message: "must be <= ev.dwell_steps_max".into(),
            });
        }

        let dr = &self.dr_event;
        if dr.start_step >= dr.end_step {
            errors.push(ConfigError {
                field: "dr_event.start_step".into(),
                message: "must be < dr_event.end_step".into(),
            });
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_preset_valid() {
        let cfg = ScenarioConfig::baseline();
        let errors = cfg.validate();
        assert!(errors.is_empty(), "baseline should be valid: {errors:?}");
    }

    #[test]
    fn from_preset_baseline() {
        let cfg = ScenarioConfig::from_preset("baseline");
        assert!(cfg.is_ok());
    }

    #[test]
    fn from_preset_unknown() {
        let err = ScenarioConfig::from_preset("nonexistent");
        assert!(err.is_err());
        let e = err.unwrap_err();
        assert!(e.message.contains("unknown preset"));
    }

    #[test]
    fn valid_toml_parses() {
        let toml = r#"
[simulation]
steps_per_day = 48
days = 2
seed = 99
imbalance_price_per_kwh = 0.15

[baseload]
base_kw = 1.0
amp_kw = 0.5
phase_rad = 0.0
noise_std = 0.1

[solar]
model = "ar1"
kw_peak = 8.0
sunrise_idx = 12
sunset_idx = 36
noise_std = 0.05
alpha = 0.85
cloud_noise_std = 0.25

[battery]
capacity_kwh = 15.0
initial_soc = 0.3
max_charge_kw = 7.0
max_discharge_kw = 7.0
eta_charge = 0.92
eta_discharge = 0.92

[ev]
max_charge_kw = 11.0
demand_kwh_min = 5.0
demand_kwh_max = 20.0
dwell_steps_min = 4
dwell_steps_max = 16

[feeder]
max_import_kw = 10.0
max_export_kw = 8.0

[dr_event]
start_step = 34
end_step = 42
requested_reduction_kw = 2.0
"#;
        let cfg = ScenarioConfig::from_toml_str(toml);
        assert!(cfg.is_ok(), "valid TOML should parse: {:?}", cfg.err());
        let cfg = cfg.ok();
        assert_eq!(cfg.as_ref().map(|c| c.simulation.steps_per_day), Some(48));
        assert_eq!(cfg.as_ref().map(|c| c.simulation.days), Some(2));
        assert_eq!(cfg.as_ref().map(|c| &*c.solar.model), Some("ar1"));
    }

    #[test]
    fn invalid_toml_unknown_field() {
        let toml = r#"
[simulation]
steps_per_day = 24
bogus_field = true
"#;
        let result = ScenarioConfig::from_toml_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn validation_catches_zero_steps() {
        let mut cfg = ScenarioConfig::baseline();
        cfg.simulation.steps_per_day = 0;
        let errors = cfg.validate();
        assert!(errors.iter().any(|e| e.field == "simulation.steps_per_day"));
    }

    #[test]
    fn validation_catches_invalid_soc() {
        let mut cfg = ScenarioConfig::baseline();
        cfg.battery.initial_soc = 1.5;
        let errors = cfg.validate();
        assert!(errors.iter().any(|e| e.field == "battery.initial_soc"));
    }

    #[test]
    fn validation_catches_bad_controller() {
        let mut cfg = ScenarioConfig::baseline();
        cfg.simulation.controller = "bogus".to_string();
        let errors = cfg.validate();
        assert!(errors.iter().any(|e| e.field == "simulation.controller"));
    }

    #[test]
    fn validation_accepts_greedy_controller() {
        let mut cfg = ScenarioConfig::baseline();
        cfg.simulation.controller = "greedy".to_string();
        let errors = cfg.validate();
        assert!(
            errors.is_empty(),
            "greedy controller should be valid: {errors:?}"
        );
    }

    #[test]
    fn validation_catches_bad_solar_model() {
        let mut cfg = ScenarioConfig::baseline();
        cfg.solar.model = "v3".to_string();
        let errors = cfg.validate();
        assert!(errors.iter().any(|e| e.field == "solar.model"));
    }

    #[test]
    fn all_presets_are_valid() {
        for name in ScenarioConfig::PRESETS {
            let cfg = ScenarioConfig::from_preset(name);
            assert!(cfg.is_ok(), "preset \"{name}\" should load");
            let errors = cfg.as_ref().map(|c| c.validate()).unwrap_or_default();
            assert!(
                errors.is_empty(),
                "preset \"{name}\" should be valid: {errors:?}"
            );
        }
    }

    #[test]
    fn high_solar_has_larger_pv() {
        let base = ScenarioConfig::baseline();
        let high = ScenarioConfig::high_solar();
        assert!(high.solar.kw_peak > base.solar.kw_peak);
        assert_eq!(high.solar.model, "ar1");
    }

    #[test]
    fn dr_stress_has_tighter_limits() {
        let base = ScenarioConfig::baseline();
        let dr = ScenarioConfig::dr_stress();
        assert!(dr.feeder.max_import_kw < base.feeder.max_import_kw);
        assert!(dr.dr_event.requested_reduction_kw > base.dr_event.requested_reduction_kw);
    }

    #[test]
    fn partial_toml_uses_defaults() {
        let toml = r#"
[simulation]
seed = 99
"#;
        let cfg = ScenarioConfig::from_toml_str(toml);
        assert!(cfg.is_ok());
        let cfg = cfg.ok();
        // seed overridden
        assert_eq!(cfg.as_ref().map(|c| c.simulation.seed), Some(99));
        // steps_per_day kept default
        assert_eq!(cfg.as_ref().map(|c| c.simulation.steps_per_day), Some(24));
        // solar kept default
        assert_eq!(cfg.as_ref().map(|c| c.solar.kw_peak), Some(5.0));
    }
}
