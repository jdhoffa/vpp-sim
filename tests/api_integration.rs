use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

const V1_KEYS: &[&str] = &[
    "timestep",
    "time_hr",
    "target_kw",
    "feeder_kw",
    "tracking_error_kw",
    "baseload_kw",
    "solar_kw",
    "ev_requested_kw",
    "ev_dispatched_kw",
    "battery_kw",
    "battery_soc",
    "dr_requested_kw",
    "dr_achieved_kw",
    "limit_ok",
];

struct ChildGuard {
    child: Child,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn api_state_and_telemetry_have_v1_schema_and_http_200() {
    let addr = allocate_bind_addr();
    let _child = spawn_api_process(&addr);

    wait_for_server(&addr, Duration::from_secs(8));

    let (state_status, state_body) =
        http_get(&addr, "/state").expect("/state request should succeed");
    assert_eq!(state_status, 200);

    let state: Value = serde_json::from_str(&state_body).expect("state body should be JSON object");
    let state_obj = state.as_object().expect("state should be an object");
    assert_has_v1_keys(state_obj);
    assert_eq!(state_obj.get("timestep").and_then(Value::as_u64), Some(23));

    let (telemetry_status, telemetry_body) =
        http_get(&addr, "/telemetry?from=2&to=4").expect("/telemetry request should succeed");
    assert_eq!(telemetry_status, 200);

    let telemetry: Value =
        serde_json::from_str(&telemetry_body).expect("telemetry body should be JSON array");
    let rows = telemetry.as_array().expect("telemetry should be an array");
    assert_eq!(rows.len(), 3);

    for row in rows {
        let row_obj = row.as_object().expect("row should be an object");
        assert_has_v1_keys(row_obj);
    }

    let first_timestep = rows[0]
        .as_object()
        .and_then(|obj| obj.get("timestep"))
        .and_then(Value::as_u64);
    let last_timestep = rows[rows.len() - 1]
        .as_object()
        .and_then(|obj| obj.get("timestep"))
        .and_then(Value::as_u64);

    assert_eq!(first_timestep, Some(2));
    assert_eq!(last_timestep, Some(4));
}

fn allocate_bind_addr() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral port bind should succeed");
    let addr = listener
        .local_addr()
        .expect("local_addr should be available")
        .to_string();
    drop(listener);
    addr
}

fn spawn_api_process(bind_addr: &str) -> ChildGuard {
    let child = Command::new(env!("CARGO_BIN_EXE_vpp-sim"))
        .args(["--preset", "demo", "--api-bind", bind_addr])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("vpp-sim process should spawn");

    ChildGuard { child }
}

fn wait_for_server(bind_addr: &str, timeout: Duration) {
    let start = Instant::now();
    loop {
        if let Ok((status, _)) = http_get(bind_addr, "/state") {
            if status == 200 {
                return;
            }
        }

        if start.elapsed() >= timeout {
            panic!("timed out waiting for API server on {bind_addr}");
        }

        thread::sleep(Duration::from_millis(50));
    }
}

fn http_get(bind_addr: &str, path: &str) -> Result<(u16, String), String> {
    let mut stream = TcpStream::connect(bind_addr).map_err(|err| format!("connect: {err}"))?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: {bind_addr}\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("write: {err}"))?;

    let mut raw = String::new();
    stream
        .read_to_string(&mut raw)
        .map_err(|err| format!("read: {err}"))?;

    let (head, body) = raw
        .split_once("\r\n\r\n")
        .ok_or_else(|| "invalid HTTP response".to_string())?;
    let status_line = head
        .lines()
        .next()
        .ok_or_else(|| "missing status line".to_string())?;
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| "missing status code".to_string())?
        .parse::<u16>()
        .map_err(|err| format!("invalid status code: {err}"))?;

    Ok((status_code, body.to_string()))
}

fn assert_has_v1_keys(object: &serde_json::Map<String, Value>) {
    for key in V1_KEYS {
        assert!(object.contains_key(*key), "missing key: {key}");
    }
}
