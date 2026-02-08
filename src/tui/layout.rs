//! TUI layout and widget rendering.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Block, Borders, Chart, Dataset, Gauge, Paragraph};

use super::runtime::App;
use super::style;

/// Renders the full TUI frame.
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(10),   // chart
            Constraint::Length(3), // SOC gauge
            Constraint::Length(5), // status panel
            Constraint::Length(1), // footer
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_chart(frame, app, chunks[1]);
    render_soc_gauge(frame, app, chunks[2]);
    render_status(frame, app, chunks[3]);
    render_footer(frame, chunks[4]);
}

/// Header bar: preset name, timestep progress, speed, run state.
fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let state_label = if app.is_finished() {
        "DONE"
    } else if app.paused {
        "PAUSED"
    } else {
        "RUNNING"
    };

    let state_icon = if app.is_finished() {
        "■"
    } else if app.paused {
        "‖"
    } else {
        "▶"
    };

    let controller = if app.preset_name == "high_solar" {
        "greedy"
    } else {
        "naive"
    };
    // high_solar uses default controller which is naive unless overridden
    // Just show the preset name and let users know
    let _ = controller; // not used for now, keep header concise

    let header = Line::from(vec![
        Span::styled(
            " VPP-SIM ",
            Style::default()
                .fg(style::HEADER_FG)
                .bg(style::HEADER_BG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            &app.preset_name,
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " │ t={}/{} │ {}ms │ {} {} ",
            app.timestep,
            app.total_steps,
            app.tick_interval_ms(),
            state_icon,
            state_label,
        )),
    ]);
    frame.render_widget(Paragraph::new(header), area);
}

/// Feeder load vs target schedule chart.
fn render_chart(frame: &mut Frame, app: &App, area: Rect) {
    // Convert history to f64 data points for the chart
    let feeder_data: Vec<(f64, f64)> = app
        .history
        .iter()
        .map(|r| (f64::from(r.timestep as u32), f64::from(r.feeder_kw)))
        .collect();

    let target_data: Vec<(f64, f64)> = app
        .history
        .iter()
        .map(|r| (f64::from(r.timestep as u32), f64::from(r.target_kw)))
        .collect();

    let y_bounds = style::auto_bounds_y(&feeder_data, &target_data);

    let x_lo = feeder_data.first().map_or(0.0, |p| p.0);
    let x_hi = feeder_data.last().map_or(1.0, |p| p.0).max(x_lo + 1.0);

    let datasets = vec![
        Dataset::default()
            .name("Feeder")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(style::FEEDER_COLOR))
            .data(&feeder_data),
        Dataset::default()
            .name("Target")
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(style::TARGET_COLOR))
            .data(&target_data),
    ];

    let x_label_lo = format!("{}", x_lo as u32);
    let x_label_hi = format!("{}", x_hi as u32);
    let y_label_lo = format!("{:.1}", y_bounds[0]);
    let y_label_hi = format!("{:.1}", y_bounds[1]);

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(" Feeder Load vs Target Schedule ")
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("step")
                .bounds([x_lo, x_hi])
                .labels(vec![x_label_lo, x_label_hi]),
        )
        .y_axis(
            Axis::default()
                .title("kW")
                .bounds(y_bounds)
                .labels(vec![y_label_lo, y_label_hi]),
        );

    frame.render_widget(chart, area);
}

/// Battery SOC gauge with DR status indicator.
fn render_soc_gauge(frame: &mut Frame, app: &App, area: Rect) {
    let soc = app.battery_soc();
    let color = style::soc_color(soc);

    let dr_status = if app.is_dr_active() { "DR: ACTIVE" } else { "" };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(14)])
        .split(area);

    let gauge = Gauge::default()
        .block(Block::default().title(" SOC ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(color))
        .ratio(f64::from(soc).clamp(0.0, 1.0))
        .label(format!("{:.0}%", soc * 100.0));
    frame.render_widget(gauge, chunks[0]);

    let dr_color = if app.is_dr_active() {
        style::DR_ACTIVE
    } else {
        style::FOOTER_FG
    };
    let dr_widget = Paragraph::new(Line::from(Span::styled(
        dr_status,
        Style::default().fg(dr_color).add_modifier(Modifier::BOLD),
    )))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(dr_widget, chunks[1]);
}

/// Status panel showing latest device power readings and metrics.
fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let lines = if let Some(r) = app.last_result() {
        let violations: usize = app
            .history
            .iter()
            .filter(|s| !s.within_feeder_limits)
            .count();
        vec![
            Line::from(format!(
                "  base={:>6.2}  solar={:>6.2}  ev={:>6.2}  bat={:>6.2}",
                r.base_kw_after_dr, r.solar_kw, r.ev_actual_kw, r.battery_actual_kw,
            )),
            Line::from(format!(
                "  feeder={:>6.2}  target={:>6.2}  err={:>6.2}  cost={:.4}",
                r.feeder_kw, r.target_kw, r.tracking_error_kw, r.imbalance_cost,
            )),
            Line::from(format!(
                "  DR(req={:.2}, done={:.2})  violations={}",
                r.dr_requested_kw, r.dr_achieved_kw, violations,
            )),
        ]
    } else {
        vec![Line::from("  Waiting for first step...")]
    };

    let block = Block::default().title(" Status ").borders(Borders::ALL);
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Footer with keybinding hints.
fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(Span::styled(
        " q:Quit  Space:Pause  +/-:Speed  1/2/3:Preset  r:Restart",
        Style::default().fg(style::FOOTER_FG),
    )));
    frame.render_widget(footer, area);
}
