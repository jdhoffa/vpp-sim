//! Keyboard input handling for the TUI.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::runtime::App;

/// Maps a key event to an application action.
///
/// Guards on [`KeyEventKind::Press`] to avoid double-fire on some terminals.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.quit = true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit = true,
        KeyCode::Char(' ') => app.toggle_pause(),
        KeyCode::Char('+' | '=') | KeyCode::Right => app.speed_up(),
        KeyCode::Char('-') | KeyCode::Left => app.speed_down(),
        KeyCode::Char('1') => app.switch_preset("baseline"),
        KeyCode::Char('2') => app.switch_preset("high_solar"),
        KeyCode::Char('3') => app.switch_preset("dr_stress"),
        KeyCode::Char('r') => app.restart(),
        _ => {}
    }
}
