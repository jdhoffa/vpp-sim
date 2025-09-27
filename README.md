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
