# vpp-sim
[![Rust](https://github.com/jdhoffa/vpp-sim/actions/workflows/rust.yml/badge.svg)](https://github.com/jdhoffa/vpp-sim/actions/workflows/rust.yml)

The **Virtual Power Plant Simulator** is an open source project aiming to simulate a neighborhood-scale Virtual Power Plant (VPP) in real time.

The simulator models a local distribution feeder with a mix of flexible and inflexible devices, including:

- ☀️ Residential solar PV
- 🔋 Home battery storage systems
- 🚗 EV charging stations
- 💡 Flexible and baseline household demand
- 🧠 A coordinating aggregator (the "VPP")
- 🚨 Demand response events for temporary load reduction
- 📏 Feeder import/export capacity constraints
- 📊 End-of-run KPI reporting

The simulation advances in fast-forwarded, discrete time steps (e.g. 5-minute intervals), allowing users to explore different configurations and control strategies through terminal output.


## Project Status

🚧 **Work in Progress** – This repository is under active development and working towards MVP status.   

Stay tuned!

## Usage
### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (latest stable version recommended)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) (comes with Rust)

### 🧩 Running the demo simulation

Running the default binary triggers a demonstrative 24-step (1-hr interval) simulation with:

- Baseline load + solar generation
- Flexible EV charging demand
- Battery tracking control
- Feeder import/export limits
- A demand response event window
- Post-run KPI summary

```bash
cargo run --release
```

Write per-timestep telemetry to CSV (schema v1):

```bash
cargo run --release -- --telemetry-out telemetry.csv
```

#### Example output:
```
Time (Hr) 0: BaseLoad=1.35 kW, RawBase=1.35 kW, Forecast=0.79 kW, Target=0.79 kW,
SolarPV=0.00 kW, EvCharger=0.00 kW (Req=0.00, DR=0.00, Cap=0.00),
Battery=0.56 kW (SoC=44.1%), MainFeeder=0.79 kW, Error=0.00 kW,
DR(req=0.00, done=0.00), LimitOK=true
...
# demand response event active; EV/baseload may be curtailed
Time (Hr) 18: BaseLoad=0.40 kW, RawBase=1.20 kW, Forecast=0.96 kW, Target=0.79 kW,
SolarPV=0.00 kW, EvCharger=0.70 kW (Req=1.20, DR=0.70, Cap=0.70),
Battery=0.31 kW (SoC=40.2%), MainFeeder=0.79 kW, Error=0.00 kW,
DR(req=1.50, done=1.30), LimitOK=true
...

--- KPI Report ---
RMSE tracking error: 0.084 kW
Curtailment achieved: 92.5%
Feeder peak load: 3.91 kW
```

Notes:

- Example values are illustrative; exact numbers depend on random seeds and configuration.
- `LimitOK=true` indicates the feeder stayed within configured import/export limits at that timestep.
- `--telemetry-out` writes CSV columns:
  `timestep,time_hr,target_kw,feeder_kw,tracking_error_kw,baseload_kw,solar_kw,ev_requested_kw,ev_dispatched_kw,battery_kw,battery_soc,dr_requested_kw,dr_achieved_kw,limit_ok`

## Documentation
The documentation for this project can be opened locally using:
```bash
cargo doc --open
```

It contains detailed information about the architecture, modules, and usage of the simulator.

## License
This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
