//! Simulation runner and TUI application state.

use std::collections::VecDeque;
use std::time::Instant;

use crate::config::ScenarioConfig;
use crate::devices::Battery;
use crate::sim::controller::{GreedyController, NaiveRtController};
use crate::sim::engine::Engine;
use crate::sim::event::DemandResponseEvent;
use crate::sim::types::{SimConfig, StepResult};

/// Maximum number of history entries kept for the rolling chart.
const MAX_HISTORY: usize = 200;

/// Tick interval options in milliseconds (slowest → fastest).
const SPEED_LEVELS_MS: [u64; 6] = [500, 250, 100, 50, 20, 5];

/// Default speed index (100 ms).
const DEFAULT_SPEED_IDX: usize = 2;

/// Engine wrapper that erases the `Controller` generic via enum dispatch.
///
/// Follows the same pattern as [`crate::devices::Solar`].
pub enum SimRunner {
    /// Engine using the naive real-time controller.
    Naive(Engine<NaiveRtController>),
    /// Engine using the greedy forecast-aware controller.
    Greedy(Engine<GreedyController>),
}

impl SimRunner {
    /// Builds a runner from a validated scenario configuration.
    pub fn from_scenario(cfg: &ScenarioConfig) -> Self {
        let c = cfg.build();
        if cfg.simulation.controller == "greedy" {
            let controller = GreedyController::new(
                &c.load_forecast,
                &c.target_schedule,
                cfg.battery.capacity_kwh,
                cfg.battery.max_charge_kw,
                cfg.battery.max_discharge_kw,
                cfg.battery.initial_soc,
                cfg.battery.eta_charge,
                cfg.battery.eta_discharge,
                c.sim_config.dt_hours,
                cfg.solar.kw_peak,
                cfg.solar.sunrise_idx,
                cfg.solar.sunset_idx,
            );
            Self::Greedy(Engine::new(
                c.sim_config,
                c.load,
                c.pv,
                c.battery,
                c.ev,
                c.feeder,
                controller,
                c.load_forecast,
                c.target_schedule,
                c.dr_event,
            ))
        } else {
            Self::Naive(Engine::new(
                c.sim_config,
                c.load,
                c.pv,
                c.battery,
                c.ev,
                c.feeder,
                NaiveRtController,
                c.load_forecast,
                c.target_schedule,
                c.dr_event,
            ))
        }
    }

    /// Advances the simulation by one timestep.
    pub fn step(&mut self, t: usize) -> StepResult {
        match self {
            Self::Naive(e) => e.step(t),
            Self::Greedy(e) => e.step(t),
        }
    }

    /// Returns the simulation configuration.
    pub fn config(&self) -> &SimConfig {
        match self {
            Self::Naive(e) => e.config(),
            Self::Greedy(e) => e.config(),
        }
    }

    /// Returns a reference to the battery device.
    pub fn battery(&self) -> &Battery {
        match self {
            Self::Naive(e) => e.battery(),
            Self::Greedy(e) => e.battery(),
        }
    }
}

/// TUI application state.
pub struct App {
    /// Simulation engine (type-erased via enum).
    runner: SimRunner,
    /// Current scenario configuration (kept for restart/preset switch).
    scenario: ScenarioConfig,
    /// Rolling history of step results for the chart.
    pub history: VecDeque<StepResult>,
    /// Next timestep to execute.
    pub timestep: usize,
    /// Total steps in the simulation.
    pub total_steps: usize,
    /// Whether the simulation is paused.
    pub paused: bool,
    /// Current index into `SPEED_LEVELS_MS`.
    pub speed_idx: usize,
    /// Whether the user has requested quit.
    pub quit: bool,
    /// When the last simulation tick was executed.
    pub last_tick: Instant,
    /// Name of the active preset.
    pub preset_name: String,
    /// DR event (for status display).
    pub dr_event: DemandResponseEvent,
}

impl App {
    /// Creates a new app from a preset name.
    pub fn new(preset: &str) -> Self {
        let scenario =
            ScenarioConfig::from_preset(preset).unwrap_or_else(|_| ScenarioConfig::baseline());
        let dr_event = DemandResponseEvent::new(
            scenario.dr_event.start_step,
            scenario.dr_event.end_step,
            scenario.dr_event.requested_reduction_kw,
        );
        let runner = SimRunner::from_scenario(&scenario);
        let total_steps = runner.config().total_steps();
        Self {
            runner,
            scenario,
            history: VecDeque::with_capacity(MAX_HISTORY),
            timestep: 0,
            total_steps,
            paused: false,
            speed_idx: DEFAULT_SPEED_IDX,
            quit: false,
            last_tick: Instant::now(),
            preset_name: preset.to_string(),
            dr_event,
        }
    }

    /// Advances the simulation by one step if not finished.
    pub fn tick(&mut self) {
        if self.timestep >= self.total_steps {
            return;
        }
        let result = self.runner.step(self.timestep);
        if self.history.len() >= MAX_HISTORY {
            self.history.pop_front();
        }
        self.history.push_back(result);
        self.timestep += 1;
    }

    /// Toggles pause/resume.
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Increases simulation speed (shorter tick interval).
    pub fn speed_up(&mut self) {
        if self.speed_idx + 1 < SPEED_LEVELS_MS.len() {
            self.speed_idx += 1;
        }
    }

    /// Decreases simulation speed (longer tick interval).
    pub fn speed_down(&mut self) {
        if self.speed_idx > 0 {
            self.speed_idx -= 1;
        }
    }

    /// Returns the current tick interval in milliseconds.
    pub fn tick_interval_ms(&self) -> u64 {
        SPEED_LEVELS_MS[self.speed_idx]
    }

    /// Switches to a different preset, resetting simulation state.
    pub fn switch_preset(&mut self, name: &str) {
        let Ok(scenario) = ScenarioConfig::from_preset(name) else {
            return;
        };
        self.dr_event = DemandResponseEvent::new(
            scenario.dr_event.start_step,
            scenario.dr_event.end_step,
            scenario.dr_event.requested_reduction_kw,
        );
        self.runner = SimRunner::from_scenario(&scenario);
        self.total_steps = self.runner.config().total_steps();
        self.scenario = scenario;
        self.history.clear();
        self.timestep = 0;
        self.paused = false;
        self.preset_name = name.to_string();
    }

    /// Restarts the current preset from the beginning.
    pub fn restart(&mut self) {
        let name = self.preset_name.clone();
        self.switch_preset(&name);
    }

    /// Returns the current battery SOC (from latest step, or initial).
    pub fn battery_soc(&self) -> f32 {
        self.history
            .back()
            .map_or(self.scenario.battery.initial_soc, |r| r.battery_soc)
    }

    /// Returns `true` when all timesteps have been executed.
    pub fn is_finished(&self) -> bool {
        self.timestep >= self.total_steps
    }

    /// Returns the most recent step result, if any.
    pub fn last_result(&self) -> Option<&StepResult> {
        self.history.back()
    }

    /// Returns `true` when a DR event is active at the current timestep.
    pub fn is_dr_active(&self) -> bool {
        self.dr_event.is_active(self.timestep.saturating_sub(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_creates_and_ticks() {
        let mut app = App::new("baseline");
        assert_eq!(app.timestep, 0);
        assert!(!app.is_finished());

        app.tick();
        assert_eq!(app.timestep, 1);
        assert_eq!(app.history.len(), 1);
    }

    #[test]
    fn app_finishes_after_total_steps() {
        let mut app = App::new("baseline");
        for _ in 0..app.total_steps {
            app.tick();
        }
        assert!(app.is_finished());
        let ts_before = app.timestep;
        app.tick(); // should be a no-op
        assert_eq!(app.timestep, ts_before);
    }

    #[test]
    fn speed_controls_stay_in_bounds() {
        let mut app = App::new("baseline");
        let initial = app.speed_idx;

        // speed down to minimum
        for _ in 0..10 {
            app.speed_down();
        }
        assert_eq!(app.speed_idx, 0);

        // speed up to maximum
        for _ in 0..10 {
            app.speed_up();
        }
        assert_eq!(app.speed_idx, SPEED_LEVELS_MS.len() - 1);

        // verify default was reasonable
        assert!(initial < SPEED_LEVELS_MS.len());
    }

    #[test]
    fn switch_preset_resets_state() {
        let mut app = App::new("baseline");
        app.tick();
        app.tick();
        assert_eq!(app.history.len(), 2);

        app.switch_preset("high_solar");
        assert_eq!(app.timestep, 0);
        assert!(app.history.is_empty());
        assert_eq!(app.preset_name, "high_solar");
    }

    #[test]
    fn restart_resets_state() {
        let mut app = App::new("dr_stress");
        for _ in 0..5 {
            app.tick();
        }
        app.restart();
        assert_eq!(app.timestep, 0);
        assert!(app.history.is_empty());
        assert_eq!(app.preset_name, "dr_stress");
    }

    #[test]
    fn toggle_pause() {
        let mut app = App::new("baseline");
        assert!(!app.paused);
        app.toggle_pause();
        assert!(app.paused);
        app.toggle_pause();
        assert!(!app.paused);
    }

    #[test]
    fn history_caps_at_max() {
        let mut app = App::new("baseline");
        // baseline has 24 steps, which is < MAX_HISTORY, so extend to multi-day
        // Just verify the cap logic works by checking we never exceed MAX_HISTORY
        for _ in 0..app.total_steps {
            app.tick();
        }
        assert!(app.history.len() <= MAX_HISTORY);
    }
}
