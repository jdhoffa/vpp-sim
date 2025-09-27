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

🚧 **Work in Progress** – This repository currently contains no runnable code. All designs, components, and interfaces are under active development.

Stay tuned!

## ⏱️ Running the simulation clock

The simulation clock drives the virtual power plant model by advancing in fixed time steps. It can be run using the `Clock` struct, which provides methods to advance time step-by-step or run a function at each time step until completion.

### Example

You can run the simple clock demo with: 
``` bash
cargo run
```

or include the `Clock` in your own project as follows:

```rust
use vpp_sim::sim::clock::Clock;

let mut clock = Clock::new(5);
clock.run(|t| println!("Step {}", t));
```

### Expected Outputs

For a clock configured with 5 total steps, the output will be:

```
Step 0
Step 1
Step 2
Step 3
Step 4
```

## License
This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
