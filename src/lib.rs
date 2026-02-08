//! Neighborhood-scale Virtual Power Plant simulator.

/// TOML scenario configuration and preset definitions.
pub mod config;
pub mod devices;
pub mod forecast;
/// I/O utilities for data export.
pub mod io;
/// Simulation engine, feeder, scheduling, and event modules.
pub mod sim;

/// REST API for simulation state and telemetry (feature-gated behind `api`).
#[cfg(feature = "api")]
pub mod api;
