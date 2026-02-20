use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ScenarioConfig {
    pub houses: u32,
    pub feeder_kw: f32,
    pub seed: u64,
    pub steps_per_day: usize,
    pub solar_kw_peak_per_house: f32,
    pub dr_start_step: usize,
    pub dr_end_step: usize,
    pub dr_reduction_kw_per_house: f32,
}

impl Default for ScenarioConfig {
    fn default() -> Self {
        Self {
            houses: 1,
            feeder_kw: 5.0,
            seed: 42,
            steps_per_day: 24,
            solar_kw_peak_per_house: 5.0,
            dr_start_step: 17,
            dr_end_step: 21,
            dr_reduction_kw_per_house: 1.5,
        }
    }
}

impl ScenarioConfig {
    pub fn from_path(path: &Path) -> Result<Self, String> {
        let resolved_path = resolve_scenario_path(path);
        let raw = fs::read_to_string(&resolved_path).map_err(|err| {
            format!(
                "failed to read scenario `{}`: {err}",
                resolved_path.display()
            )
        })?;

        let ext = resolved_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let pairs = match ext {
            "toml" => parse_flat_toml_table(&raw).map_err(|err| {
                format!(
                    "invalid TOML in scenario `{}`: {err}",
                    resolved_path.display()
                )
            })?,
            _ => {
                return Err(format!(
                    "unsupported scenario format for `{}` (expected .toml)",
                    resolved_path.display()
                ));
            }
        };

        Self::from_kv_pairs(&pairs)
            .map_err(|err| format!("invalid scenario `{}`: {err}", resolved_path.display()))
    }

    pub fn from_preset(name: &str) -> Result<Self, String> {
        let scenario_path = PathBuf::from("scenarios").join(format!("{name}.toml"));
        if scenario_path.exists() {
            return Self::from_path(&scenario_path);
        }

        match name {
            "demo" => Ok(Self::default()),
            _ => Err(format!(
                "invalid value for `preset`: unknown preset `{name}` (expected `demo` or file `{}`)",
                scenario_path.display()
            )),
        }
    }

    fn from_kv_pairs(obj: &[(String, String)]) -> Result<Self, String> {
        for (key, _) in obj {
            match key.as_str() {
                "houses"
                | "feeder_kw"
                | "seed"
                | "steps_per_day"
                | "solar_kw_peak_per_house"
                | "dr_start_step"
                | "dr_end_step"
                | "dr_reduction_kw_per_house" => {}
                _ => return Err(format!("at `$.{key}`: unknown key")),
            }
        }

        let houses = parse_u32(find_value(obj, "houses"), "$.houses", 1)?;
        let feeder_kw = parse_f32(find_value(obj, "feeder_kw"), "$.feeder_kw", 5.0)?;
        let seed = parse_u64(find_value(obj, "seed"), "$.seed", 42)?;
        let steps_per_day = parse_usize(find_value(obj, "steps_per_day"), "$.steps_per_day", 24)?;
        let solar_kw_peak_per_house = parse_f32(
            find_value(obj, "solar_kw_peak_per_house"),
            "$.solar_kw_peak_per_house",
            5.0,
        )?;
        let dr_start_step = parse_usize(find_value(obj, "dr_start_step"), "$.dr_start_step", 17)?;
        let dr_end_step = parse_usize(find_value(obj, "dr_end_step"), "$.dr_end_step", 21)?;
        let dr_reduction_kw_per_house = parse_f32(
            find_value(obj, "dr_reduction_kw_per_house"),
            "$.dr_reduction_kw_per_house",
            1.5,
        )?;

        if houses == 0 {
            return Err("at `$.houses`: must be > 0".to_string());
        }
        if feeder_kw <= 0.0 {
            return Err("at `$.feeder_kw`: must be > 0".to_string());
        }
        if steps_per_day == 0 {
            return Err("at `$.steps_per_day`: must be > 0".to_string());
        }
        if solar_kw_peak_per_house < 0.0 {
            return Err("at `$.solar_kw_peak_per_house`: must be >= 0".to_string());
        }
        if dr_start_step >= steps_per_day {
            return Err("at `$.dr_start_step`: must be < steps_per_day".to_string());
        }
        if dr_end_step > steps_per_day {
            return Err("at `$.dr_end_step`: must be <= steps_per_day".to_string());
        }
        if dr_start_step >= dr_end_step {
            return Err("at `$.dr_start_step`: must be < dr_end_step".to_string());
        }
        if dr_reduction_kw_per_house < 0.0 {
            return Err("at `$.dr_reduction_kw_per_house`: must be >= 0".to_string());
        }

        Ok(Self {
            houses,
            feeder_kw,
            seed,
            steps_per_day,
            solar_kw_peak_per_house,
            dr_start_step,
            dr_end_step,
            dr_reduction_kw_per_house,
        })
    }
}

fn resolve_scenario_path(path: &Path) -> PathBuf {
    if path.exists() {
        return path.to_path_buf();
    }

    let fallback = PathBuf::from("scenarios").join(path);
    if fallback.exists() {
        fallback
    } else {
        path.to_path_buf()
    }
}

fn find_value<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find_map(|(k, v)| if k == key { Some(v.as_str()) } else { None })
}

fn parse_u32(value: Option<&str>, path: &str, default: u32) -> Result<u32, String> {
    let Some(v) = value else {
        return Ok(default);
    };
    let n = v
        .parse::<u64>()
        .map_err(|_| format!("at `{path}`: expected unsigned integer"))?;
    u32::try_from(n).map_err(|_| format!("at `{path}`: value out of range for u32"))
}

fn parse_u64(value: Option<&str>, path: &str, default: u64) -> Result<u64, String> {
    let Some(v) = value else {
        return Ok(default);
    };
    v.parse::<u64>()
        .map_err(|_| format!("at `{path}`: expected unsigned integer"))
}

fn parse_usize(value: Option<&str>, path: &str, default: usize) -> Result<usize, String> {
    let Some(v) = value else {
        return Ok(default);
    };
    let n = v
        .parse::<u64>()
        .map_err(|_| format!("at `{path}`: expected unsigned integer"))?;
    usize::try_from(n).map_err(|_| format!("at `{path}`: value out of range for usize"))
}

fn parse_f32(value: Option<&str>, path: &str, default: f32) -> Result<f32, String> {
    let Some(v) = value else {
        return Ok(default);
    };
    let n = v
        .parse::<f64>()
        .map_err(|_| format!("at `{path}`: expected number"))?;
    if !n.is_finite() {
        return Err(format!("at `{path}`: expected finite number"));
    }
    Ok(n as f32)
}

fn parse_flat_toml_table(raw: &str) -> Result<Vec<(String, String)>, String> {
    let value: toml::Value = raw
        .parse::<toml::Value>()
        .map_err(|err| format!("failed to parse TOML: {err}"))?;
    let table = value
        .as_table()
        .ok_or_else(|| "expected top-level TOML table".to_string())?;

    let mut pairs = Vec::with_capacity(table.len());
    for (key, value) in table {
        let as_string = toml_value_to_numeric_string(value, key)?;
        pairs.push((key.clone(), as_string));
    }
    Ok(pairs)
}

fn toml_value_to_numeric_string(value: &toml::Value, key: &str) -> Result<String, String> {
    match value {
        toml::Value::Integer(n) => Ok(n.to_string()),
        toml::Value::Float(n) => Ok(n.to_string()),
        _ => Err(format!(
            "at `$.{key}`: expected numeric value (integer or float)"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{ScenarioConfig, parse_flat_toml_table};
    use std::path::Path;

    #[test]
    fn scenario_validation_includes_offending_key_path() {
        let value = vec![("houses".to_string(), "0".to_string())];
        let err = ScenarioConfig::from_kv_pairs(&value).expect_err("must fail");
        assert!(err.contains("$.houses"));
    }

    #[test]
    fn unknown_key_reports_path() {
        let value = vec![("bad_key".to_string(), "1".to_string())];
        let err = ScenarioConfig::from_kv_pairs(&value).expect_err("must fail");
        assert!(err.contains("$.bad_key"));
    }

    #[test]
    fn parses_flat_toml_table() {
        let pairs =
            parse_flat_toml_table("houses = 2\nfeeder_kw = 10.5\nseed = 9").expect("toml parse");
        assert!(pairs.iter().any(|(k, v)| k == "houses" && v == "2"));
        assert!(pairs.iter().any(|(k, v)| k == "feeder_kw" && v == "10.5"));
    }

    #[test]
    fn bare_filename_resolves_from_scenarios_dir() {
        let cfg = ScenarioConfig::from_path(Path::new("baseline.toml"))
            .expect("baseline preset from scenarios dir should load");
        assert!(cfg.houses > 0);
    }
}
