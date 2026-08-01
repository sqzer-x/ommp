use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme::Theme;

#[derive(Debug, Clone, PartialEq)]
pub enum PlaylistModalMode {
    List,
    Create,
    Rename,
}

pub fn render_playlist_modal(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    mode: &PlaylistModalMode,
    input: &str,
    app: &App,
    theme: &Theme,
) {
    let modal = centered_rect(45, 45, area);

    frame.render_widget(Clear, modal);

    let title = match mode {
        PlaylistModalMode::List => " Playlist ",
        PlaylistModalMode::Create => " New Playlist ",
        PlaylistModalMode::Rename => " Rename Playlist ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(title)
        .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    match mode {
        PlaylistModalMode::Create | PlaylistModalMode::Rename => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Length(1),
                    Constraint::Min(1),
                ])
                .split(inner);

            let prompt = if *mode == PlaylistModalMode::Create { "Name:" } else { "New name:" };
            let input_line = Line::from(vec![
                Span::styled(
                    format!(" {} ", prompt),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                Span::styled(input, Style::default().fg(Color::White)),
                Span::styled("_", Style::default().fg(Color::Yellow).add_modifier(Modifier::SLOW_BLINK)),
            ]);
            frame.render_widget(Paragraph::new(input_line), chunks[0]);

            let hint = Line::from(Span::styled(
                " Enter: confirm  Esc: cancel",
                Style::default().fg(Color::DarkGray),
            ));
            frame.render_widget(Paragraph::new(hint), chunks[2]);
        }
        PlaylistModalMode::List => {
            if app.playlists.is_empty() {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(1)])
                    .split(inner);

                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        "  No playlists. Press 'a' to create.",
                        Style::default().fg(Color::DarkGray),
                    ))),
                    chunks[0],
                );
                let hint = Line::from(Span::styled(
                    " a: add  Esc: close ",
                    Style::default().fg(Color::DarkGray),
                ));
                frame.render_widget(Paragraph::new(hint), chunks[1]);
                return;
            }

            let current_track_idx = app.queue.current_index
                .and_then(|qi| app.queue.tracks.get(qi).copied());

            let items: Vec<ListItem> = app
                .playlists
                .iter()
                .enumerate()
                .map(|(i, pl)| {
                    let is_selected = i == selected;
                    let already_in = current_track_idx
                        .is_some_and(|ti| pl.tracks.contains(&ti));

                    let check = if already_in { "\u{F00C} " } else { "  " };
                    let icon = "\u{F005} ";

                    let style = if is_selected {
                        Style::default()
                            .bg(theme.highlight_bg)
                            .fg(theme.highlight_fg)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg)
                    };

                    let check_style = if is_selected {
                        style
                    } else if already_in {
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };

                    let count_style = if is_selected {
                        style
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };

                    ListItem::new(Line::from(vec![
                        Span::styled(check, check_style),
                        Span::styled(icon, if is_selected { style } else { Style::default().fg(Color::Yellow) }),
                        Span::styled(&pl.name, style),
                        Span::styled(format!(" ({})", pl.tracks.len()), count_style),
                    ]))
                })
                .collect();

            let hint = Line::from(Span::styled(
                " Enter: toggle  a: add  d: delete  r: rename  Esc: close ",
                Style::default().fg(Color::DarkGray),
            ));

            let list = List::new(items).block(
                Block::default().title_bottom(hint)
            );
            frame.render_widget(list, inner);
        }
    }
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
