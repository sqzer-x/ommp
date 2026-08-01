use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::ui::theme::Theme;

const KEYBINDINGS: &[(&str, &str)] = &[
    ("Ctrl+E, s", "Search"),
    ("Ctrl+E, h", "Help (this modal)"),
    ("Ctrl+E, r", "Resize mode"),
    ("Ctrl+E, i", "About OMMP"),
    ("Ctrl+E, l", "Sync library"),
    ("", ""),
    ("Space", "Play / Pause"),
    ("n / N", "Next / Previous track"),
    ("+ / -", "Volume up / down"),
    ("\u{2192} / \u{2190}", "Seek forward / backward"),
    ("s", "Toggle shuffle"),
    ("r", "Cycle repeat mode"),
    ("b", "Add to playlist"),
    ("", ""),
    ("1-7", "Switch tab"),
    ("Tab / Shift+Tab", "Cycle pane focus"),
    ("j / k", "Navigate list"),
    ("g / G", "Jump to first / last"),
    ("Enter", "Select / Activate"),
    ("d", "Remove from queue"),
    ("c", "Clear queue"),
    ("q", "Quit"),
];

pub fn render_help_modal(frame: &mut Frame, area: Rect, theme: &Theme) {
    let modal = centered_rect(50, 70, area);

    frame.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Keybindings ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let lines: Vec<Line> = KEYBINDINGS
        .iter()
        .map(|(key, desc)| {
            if key.is_empty() {
                Line::from("")
            } else {
                Line::from(vec![
                    Span::styled(
                        format!("  {:20}", key),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        desc.to_string(),
                        Style::default().fg(theme.fg),
                    ),
                ])
            }
        })
        .collect();

    let help_text = Paragraph::new(lines);
    frame.render_widget(help_text, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
