use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::ui::text::centered_rect;
use crate::ui::theme::Theme;

/// Left column: playback and global commands.
const PLAYBACK: &[(&str, &str)] = &[
    ("Space", "Play / Pause"),
    ("n / N", "Next / Previous track"),
    ("+ / -", "Volume up / down"),
    ("\u{2192} / \u{2190}", "Seek forward / back"),
    ("s", "Toggle shuffle"),
    ("r", "Cycle repeat mode"),
    ("b", "Add to playlist"),
    ("p", "Info panel: clock / art"),
    ("", ""),
    ("Ctrl+S", "Search"),
    ("Ctrl+H", "Help (this modal)"),
    ("Ctrl+A", "About OMMP"),
    ("Ctrl+L", "Sync library"),
];

/// Right column: navigation and the queue.
const NAVIGATION: &[(&str, &str)] = &[
    ("1-7", "Switch tab"),
    ("Tab / Shift+Tab", "Cycle pane focus"),
    ("h / l", "Focus prev / next pane"),
    ("j / k", "Navigate list"),
    ("g / G", "Jump to first / last"),
    ("Enter", "Select / Activate"),
    ("", ""),
    ("d", "Remove from queue"),
    ("c", "Clear queue"),
    ("", ""),
    ("Ctrl+drag border", "Resize panels"),
    ("Double-click", "Play queue track"),
    ("", ""),
    ("Esc", "Close modal"),
    ("q / Ctrl+C", "Quit"),
];

pub fn render_help_modal(frame: &mut Frame, area: Rect, theme: &Theme) {
    // Two columns: a single list of 27 entries does not fit an 80x24 terminal,
    // and there is no scrolling — the tail, including `q: Quit`, was simply cut.
    let modal = centered_rect(72, 70, area);

    frame.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Keybindings ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    for (bindings, area) in [(PLAYBACK, columns[0]), (NAVIGATION, columns[1])] {
        frame.render_widget(Paragraph::new(render_column(bindings, theme)), area);
    }
}

fn render_column(bindings: &[(&str, &str)], theme: &Theme) -> Vec<Line<'static>> {
    bindings
        .iter()
        .map(|(key, desc)| {
            if key.is_empty() {
                return Line::from("");
            }
            Line::from(vec![
                Span::styled(
                    format!("  {:16}", key),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                Span::styled(desc.to_string(), Style::default().fg(theme.fg)),
            ])
        })
        .collect()
}
