//! Color constants and auto-scaling helpers for the TUI.

use ratatui::style::Color;

/// Feeder load line color.
pub const FEEDER_COLOR: Color = Color::Cyan;
/// Target schedule line color.
pub const TARGET_COLOR: Color = Color::DarkGray;
/// SOC gauge color when high (>= 50%).
pub const SOC_HIGH: Color = Color::Green;
/// SOC gauge color when medium (>= 20%).
pub const SOC_MID: Color = Color::Yellow;
/// SOC gauge color when low (< 20%).
pub const SOC_LOW: Color = Color::Red;
/// Header bar foreground.
pub const HEADER_FG: Color = Color::White;
/// Header bar background.
pub const HEADER_BG: Color = Color::DarkGray;
/// Footer help text color.
pub const FOOTER_FG: Color = Color::DarkGray;
/// DR active indicator color.
pub const DR_ACTIVE: Color = Color::Magenta;

/// Returns a color based on the battery state of charge.
pub fn soc_color(soc: f32) -> Color {
    if soc >= 0.5 {
        SOC_HIGH
    } else if soc >= 0.2 {
        SOC_MID
    } else {
        SOC_LOW
    }
}

/// Computes Y-axis bounds from chart data points with 10% padding.
pub fn auto_bounds_y(feeder: &[(f64, f64)], target: &[(f64, f64)]) -> [f64; 2] {
    let all = feeder.iter().chain(target.iter()).map(|&(_, y)| y);
    let min = all.clone().fold(f64::INFINITY, f64::min);
    let max = all.fold(f64::NEG_INFINITY, f64::max);
    if !min.is_finite() || !max.is_finite() {
        return [-1.0, 1.0];
    }
    let range = (max - min).max(0.1);
    let pad = range * 0.1;
    [min - pad, max + pad]
}
