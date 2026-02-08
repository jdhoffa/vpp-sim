//! Live terminal UI for interactive simulation visualization.
//!
//! Feature-gated behind `tui`. Launch with `--tui` on the CLI.

mod controls;
mod layout;
/// Simulation runner and application state.
pub mod runtime;
mod style;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use runtime::App;

/// Launches the TUI event loop for the given preset.
///
/// Sets up the terminal (raw mode, alternate screen), runs the event loop,
/// and restores the terminal on exit.
pub fn run(preset: &str) {
    enable_raw_mode().unwrap_or_else(|e| {
        eprintln!("error: failed to enable raw mode: {e}");
        std::process::exit(1);
    });

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).unwrap_or_else(|e| {
        let _ = disable_raw_mode();
        eprintln!("error: failed to enter alternate screen: {e}");
        std::process::exit(1);
    });

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap_or_else(|e| {
        let _ = disable_raw_mode();
        eprintln!("error: failed to create terminal: {e}");
        std::process::exit(1);
    });

    let mut app = App::new(preset);
    let result = event_loop(&mut terminal, &mut app);

    // Teardown — always restore terminal state
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    if let Err(e) = result {
        eprintln!("error: TUI crashed: {e}");
        std::process::exit(1);
    }
}

/// Core event loop: poll input, advance simulation, draw.
fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| layout::render(frame, app))?;

        if app.quit {
            return Ok(());
        }

        let timeout = Duration::from_millis(app.tick_interval_ms());
        let deadline = app.last_tick + timeout;
        let now = Instant::now();
        let poll_timeout = deadline.saturating_duration_since(now);

        if event::poll(poll_timeout)? {
            if let Event::Key(key) = event::read()? {
                controls::handle_key(app, key);
            }
        }

        if app.last_tick.elapsed() >= timeout && !app.paused && !app.is_finished() {
            app.tick();
            app.last_tick = Instant::now();
        }
    }
}
