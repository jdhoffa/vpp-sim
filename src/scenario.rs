use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ScenarioConfig {
    pub houses: u32,
    pub feeder_kw: f32,
    pub seed: u64,
    pub steps_per_day: usize,
}

impl Default for ScenarioConfig {
    fn default() -> Self {
        Self {
            houses: 1,
            feeder_kw: 5.0,
            seed: 42,
            steps_per_day: 24,
        }
    }
}

impl ScenarioConfig {
    pub fn from_json_path(path: &Path) -> Result<Self, String> {
        let raw = fs::read_to_string(path)
            .map_err(|err| format!("failed to read scenario `{}`: {err}", path.display()))?;
        let pairs = parse_flat_json_object(&raw)
            .map_err(|err| format!("invalid JSON in scenario `{}`: {err}", path.display()))?;
        Self::from_kv_pairs(&pairs)
            .map_err(|err| format!("invalid scenario `{}`: {err}", path.display()))
    }

    pub fn from_preset(name: &str) -> Result<Self, String> {
        let scenario_path = PathBuf::from("scenarios").join(format!("{name}.json"));
        if scenario_path.exists() {
            return Self::from_json_path(&scenario_path);
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
                "houses" | "feeder_kw" | "seed" | "steps_per_day" => {}
                _ => return Err(format!("at `$.{key}`: unknown key")),
            }
        }

        let houses = parse_u32(find_value(obj, "houses"), "$.houses", 1)?;
        let feeder_kw = parse_f32(find_value(obj, "feeder_kw"), "$.feeder_kw", 5.0)?;
        let seed = parse_u64(find_value(obj, "seed"), "$.seed", 42)?;
        let steps_per_day = parse_usize(find_value(obj, "steps_per_day"), "$.steps_per_day", 24)?;

        if houses == 0 {
            return Err("at `$.houses`: must be > 0".to_string());
        }
        if feeder_kw <= 0.0 {
            return Err("at `$.feeder_kw`: must be > 0".to_string());
        }
        if steps_per_day == 0 {
            return Err("at `$.steps_per_day`: must be > 0".to_string());
        }

        Ok(Self {
            houses,
            feeder_kw,
            seed,
            steps_per_day,
        })
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

fn parse_flat_json_object(raw: &str) -> Result<Vec<(String, String)>, String> {
    let mut i = 0usize;
    let bytes = raw.as_bytes();
    skip_ws(bytes, &mut i);
    expect_char(bytes, &mut i, b'{')?;
    skip_ws(bytes, &mut i);

    let mut pairs = Vec::new();
    if i < bytes.len() && bytes[i] == b'}' {
        i += 1;
        skip_ws(bytes, &mut i);
        if i != bytes.len() {
            return Err(format!("unexpected trailing content at byte {i}"));
        }
        return Ok(pairs);
    }

    loop {
        skip_ws(bytes, &mut i);
        let key = parse_json_string(bytes, &mut i)?;
        skip_ws(bytes, &mut i);
        expect_char(bytes, &mut i, b':')?;
        skip_ws(bytes, &mut i);
        let value = parse_json_number_literal(bytes, &mut i)?;
        pairs.push((key, value));
        skip_ws(bytes, &mut i);

        if i >= bytes.len() {
            return Err("expected `,` or `}` at end of object".to_string());
        }
        match bytes[i] {
            b',' => {
                i += 1;
            }
            b'}' => {
                i += 1;
                break;
            }
            _ => return Err(format!("expected `,` or `}}` at byte {i}")),
        }
    }

    skip_ws(bytes, &mut i);
    if i != bytes.len() {
        return Err(format!("unexpected trailing content at byte {i}"));
    }
    Ok(pairs)
}

fn skip_ws(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
        *i += 1;
    }
}

fn expect_char(bytes: &[u8], i: &mut usize, ch: u8) -> Result<(), String> {
    if *i >= bytes.len() {
        return Err(format!("expected `{}` at end of input", ch as char));
    }
    if bytes[*i] != ch {
        return Err(format!("expected `{}` at byte {}", ch as char, *i));
    }
    *i += 1;
    Ok(())
}

fn parse_json_string(bytes: &[u8], i: &mut usize) -> Result<String, String> {
    expect_char(bytes, i, b'"')?;
    let start = *i;
    while *i < bytes.len() {
        let c = bytes[*i];
        if c == b'\\' {
            return Err(format!(
                "unsupported escaped string at byte {} (only plain keys are supported)",
                *i
            ));
        }
        if c == b'"' {
            let s = std::str::from_utf8(&bytes[start..*i])
                .map_err(|_| format!("invalid UTF-8 in string at byte {start}"))?;
            *i += 1;
            return Ok(s.to_string());
        }
        *i += 1;
    }
    Err("unterminated string".to_string())
}

fn parse_json_number_literal(bytes: &[u8], i: &mut usize) -> Result<String, String> {
    let start = *i;
    if *i < bytes.len() && (bytes[*i] == b'+' || bytes[*i] == b'-') {
        *i += 1;
    }
    while *i < bytes.len() {
        let c = bytes[*i];
        if c.is_ascii_digit() || c == b'.' || c == b'e' || c == b'E' || c == b'+' || c == b'-' {
            *i += 1;
        } else {
            break;
        }
    }
    if start == *i {
        return Err(format!("expected number at byte {start}"));
    }
    let value = std::str::from_utf8(&bytes[start..*i])
        .map_err(|_| format!("invalid number bytes at byte {start}"))?;
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::ScenarioConfig;

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
}
