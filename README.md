# vpp-sim
[![Rust](https://github.com/jdhoffa/vpp-sim/actions/workflows/rust.yml/badge.svg)](https://github.com/jdhoffa/vpp-sim/actions/workflows/rust.yml)

The **Virtual Power Plant Simulator** is an open source project aiming to simulate a neighborhood-scale Virtual Power Plant (VPP) in real time.

The simulator models a local distribution feeder with a mix of flexible and inflexible devices, including:

- ☀️ Residential solar PV
- 🔋 Home battery storage systems
- 🚗 EV charging stations
- 💡 Flexible and baseline household demand
- 🧠 A coordinating aggregator (the "VPP")

The simulation advances in fast-forwarded, discrete time steps (e.g. 5-minute intervals), allowing users to explore different configurations and control strategies in real time through a terminal-based user interface (TUI).


## Project Status

🚧 **Work in Progress** – This repository is under active development and working towards MVP status.   

Stay tuned!

## Usage
### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (latest stable version recommended)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) (comes with Rust)

### 🧩 Running the demo simulation

Running the default binary will trigger a demonstrative 24-step (1-hr interval) simulation with a simple baseload, solar and battery model:

```bash
cargo run --release
```

#### Expected outputs:
```
Time (Hr) 0: BaseLoad=1.35 kW, SolarPV=0.00 kW, Battery=1.35 kW (SoC=35.8%), Net=0.00 kW
Time (Hr) 1: BaseLoad=1.42 kW, SolarPV=0.00 kW, Battery=1.42 kW (SoC=20.9%), Net=0.00 kW
...
# high solar generation, battery charging
Time (Hr) 10: BaseLoad=0.39 kW, SolarPV=3.73 kW, Battery=-3.34 kW (SoC=53.6%), Net=0.00 kW
Time (Hr) 11: BaseLoad=0.18 kW, SolarPV=4.72 kW, Battery=-4.54 kW (SoC=96.7%), Net=0.00 kW
...
# no solar generation, battery discharging
Time (Hr) 20: BaseLoad=0.95 kW, SolarPV=0.00 kW, Battery=0.95 kW (SoC=76.3%), Net=0.00 kW
Time (Hr) 21: BaseLoad=1.02 kW, SolarPV=0.00 kW, Battery=1.02 kW (SoC=65.5%), Net=0.00 kW
```

## Documentation
The documentation for this project can be opened locally using:
```bash
cargo doc --open
```

It contains detailed information about the architecture, modules, and usage of the simulator.

## License
This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
