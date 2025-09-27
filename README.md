# vpp-sim
[![Rust](https://github.com/jdhoffa/vpp-sim/actions/workflows/rust.yml/badge.svg)](https://github.com/jdhoffa/vpp-sim/actions/workflows/rust.yml)

The **Virtual Power Plant Simulator** is an open source project aiming to simulate a neighborhood-scale Virtual Power Plant (VPP) in real time.

The simulator models a local distribution feeder with a mix of flexible and inflexible devices, including:

- 🏠 Residential solar PV
- 🔋 Home battery storage systems
- 🚗 EV charging stations
- 💡 Flexible and baseline household demand
- 🧠 A coordinating aggregator (the "VPP")

The simulation advances in fast-forwarded, discrete time steps (e.g. 5-minute intervals), allowing users to explore different configurations and control strategies in real time through a terminal-based user interface (TUI).


## Project Status

🚧 **Work in Progress** – This repository is under active development and working towards MVP status.   

Stay tuned!

## Usage

Running the default binary will trigger a 20-step simulation with a simple baseload model. This will output the modeled baseload demand (in kW) at each time step:

```bash
cargo run --release
```

### Expected outputs:
```
t=0, baseload_kw=1.35
t=1, baseload_kw=1.39
t=2, baseload_kw=1.46
t=3, baseload_kw=1.48
...
t=19, baseload_kw=1.22
```



## ⏱️ Running the simulation clock

The simulation clock drives the virtual power plant model by advancing in fixed time steps. It can be run using the `Clock` struct, which provides methods to advance time step-by-step or run a function at each time step until completion.

### Example

You can run the simple `Clock` with:

```rust
use vpp_sim::sim::clock::Clock;

let mut clock = Clock::new(5);
clock.run(|t| println!("Step {}", t));
```

## ⚡ Running the BaseLoad model

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

## License
This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
