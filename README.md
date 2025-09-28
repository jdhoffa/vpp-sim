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
### 🧩 Running the demo simulation

Running the default binary will trigger a demonstrative 96-step simulation (15-minute interval) with a simple baseload model. This will output the modeled baseload demand (in kW) at each time step:

```bash
cargo run --release
```

#### Expected outputs:
```
Timestep 0: BaseLoad demand = 1.353 kW, SolarPV generation = 0.000 kW, Net = 1.353 kW
Timestep 1: BaseLoad demand = 1.387 kW, SolarPV generation = 0.000 kW, Net = 1.387 kW
Timestep 2: BaseLoad demand = 1.463 kW, SolarPV generation = 0.000 kW, Net = 1.463 kW
...
# peak solar generation at noon
Timestep 48: BaseLoad demand = 0.183 kW, SolarPV generation = -5.310 kW, Net = -5.127 kW
...
Timestep 95: BaseLoad demand = 1.393 kW, SolarPV generation = 0.000 kW, Net = 1.393 kW
```


### ⏱️ Running the simulation clock

The simulation clock drives the virtual power plant model by advancing in fixed time steps. It can be run using the `Clock` struct, which provides methods to advance time step-by-step or run a function at each time step until completion.

You can run the simple `Clock` with:

```rust
use vpp_sim::sim::clock::Clock;

let mut clock = Clock::new(5);
clock.run(|t| println!("Step {}", t));
```

### ⚡ Running the BaseLoad model

The `BaseLoad` model simulates the baseline electricity consumption of a household. It can be run using the `BaseLoad` struct, which provides methods to get the load at each time step.

You can run the simple `BaseLoad` with:

```rust
use vpp_sim::sim::load::BaseLoad;

// Create a baseload with typical parameters
let mut load = BaseLoad::new(
    1.0,   // base_kw - average consumption
    0.5,   // amp_kw - daily variation
    0.0,   // phase_rad - no phase shift (minimum at midnight)
    0.05,  // noise_std - small random variation
    24,    // steps_per_day - hourly resolution
    42,    // seed - for reproducible randomness
);

// Get demand at specific time step
let demand = load.demand_kw(12); // demand at noon
```

### ☀️ Running the Solar PV model
The `SolarPV` model simulates the electricity generation from a residential solar photovoltaic system. It can be run using the `SolarPV` struct, which provides methods to get the generation at each time step based on a daylight fraction.

You can run the simple `SolarPV` with:

```rust
use vpp_sim::devices::solar::SolarPv;
// Create a solar PV system with a specified capacity
let mut pv = SolarPv::new(
    5.0,   // kw_peak - maximum output in ideal conditions
    24,    // steps_per_day - hourly resolution
    6,     // sunrise_idx - 6am sunrise
    18,    // sunset_idx - 6pm sunset
    0.05,  // noise_std - small random variation for cloud cover
    42,    // seed - for reproducible randomness
);
// Get generation at specific time step (e.g., at noon)
let generation = pv.gen_kw(12); // generation at timestep 12 (noon)
```

## License
This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
