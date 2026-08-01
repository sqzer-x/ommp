use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Gauge};
use ratatui::Frame;

use crate::app::App;
use crate::app::state::PlayState;
use crate::ui::theme::Theme;

pub fn render_progress_bar(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_unfocused));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(3),  // Play icon
            Constraint::Min(10),   // Gauge bar
            Constraint::Length(13), // Time display
        ])
        .split(inner);

    // Play icon
    let icon = match app.playback.state {
        PlayState::Playing => "\u{F04B}",  // nf-fa-play
        PlayState::Paused => "\u{F04C}",   // nf-fa-pause
        PlayState::Stopped => "\u{F04D}",  // nf-fa-stop
    };
    let icon_color = match app.playback.state {
        PlayState::Playing => Color::Rgb(80, 255, 120),
        PlayState::Paused => Color::Rgb(255, 200, 80),
        PlayState::Stopped => Color::Rgb(255, 100, 100),
    };
    let icon_widget = Paragraph::new(Line::from(Span::styled(
        format!(" {}", icon),
        Style::default().fg(icon_color),
    )));
    frame.render_widget(icon_widget, cols[0]);

    // Gauge
    let ratio = if app.playback.duration_secs > 0.0 {
        (app.playback.position_secs / app.playback.duration_secs).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let gauge = Gauge::default()
        .ratio(ratio)
        .label("")
        .gauge_style(Style::default().fg(theme.progress_filled).bg(theme.progress_empty));
    frame.render_widget(gauge, cols[1]);

    // Time
    let pos = format_time(app.playback.position_secs);
    let dur = format_time(app.playback.duration_secs);
    let time_widget = Paragraph::new(Line::from(Span::styled(
        format!(" {} / {}", pos, dur),
        Style::default().fg(Color::White),
    )));
    frame.render_widget(time_widget, cols[2]);
}

/// Returns the gauge area for mouse click seeking
pub fn progress_gauge_area(area: Rect) -> Rect {
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(13),
        ])
        .split(inner);

    cols[1]
}

fn format_time(secs: f64) -> String {
    let total = secs as u64;
    let m = total / 60;
    let s = total % 60;
    format!("{}:{:02}", m, s)
}
