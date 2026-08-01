use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme::Theme;

const HOVER_BG: Color = Color::Indexed(238);

#[allow(clippy::too_many_arguments)]
pub fn render_search_modal(
    frame: &mut Frame,
    area: Rect,
    input: &str,
    results: &[usize],
    selected: usize,
    scroll: usize,
    hover_row: Option<usize>,
    app: &App,
    theme: &Theme,
) -> (usize, Rect) {
    let modal = centered_rect(50, 60, area);

    frame.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Search ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    // Split inner: input(1) + separator(1) + results(rest)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // input line
            Constraint::Length(1), // separator
            Constraint::Min(1),   // results
        ])
        .split(inner);

    // Input line with cursor
    let input_line = Line::from(vec![
        Span::styled(" > ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(input, Style::default().fg(Color::White)),
        Span::styled("_", Style::default().fg(Color::Cyan).add_modifier(Modifier::SLOW_BLINK)),
    ]);
    frame.render_widget(Paragraph::new(input_line), chunks[0]);

    // Separator
    let sep = "─".repeat(chunks[1].width as usize);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(sep, Style::default().fg(Color::DarkGray)))),
        chunks[1],
    );

    // Results list
    let result_height = chunks[2].height as usize;
    let result_width = chunks[2].width as usize;

    if results.is_empty() {
        let msg = if input.is_empty() {
            "Type to search..."
        } else {
            "No results found"
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("  {}", msg),
                Style::default().fg(Color::DarkGray),
            ))),
            chunks[2],
        );
    } else {
        let items: Vec<ListItem> = results
            .iter()
            .enumerate()
            .skip(scroll)
            .take(result_height)
            .map(|(i, &track_idx)| {
                let track = &app.library.tracks[track_idx];
                let is_selected = i == selected;
                let is_hovered = hover_row == Some(i);

                let artist = track.display_artist();
                let title_w = (result_width * 55 / 100).max(4);
                let artist_w = result_width.saturating_sub(title_w + 3); // 3 = prefix + gap

                let title_fitted = fit_to_width(&track.title, title_w);
                let artist_fitted = fit_to_width(artist, artist_w);

                let (style, artist_style) = if is_selected {
                    let s = Style::default()
                        .bg(theme.highlight_bg)
                        .fg(theme.highlight_fg)
                        .add_modifier(Modifier::BOLD);
                    (s, s)
                } else if is_hovered {
                    (
                        Style::default().fg(theme.fg).bg(HOVER_BG),
                        Style::default().fg(Color::Gray).bg(HOVER_BG),
                    )
                } else {
                    (
                        Style::default().fg(theme.fg),
                        Style::default().fg(Color::Gray),
                    )
                };

                let prefix = if is_selected { " > " } else { "   " };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(title_fitted, style),
                    Span::styled(artist_fitted, artist_style),
                ]))
            })
            .collect();

        let count_info = format!(" {}/{} ", results.len(), app.library.tracks.len());
        let list = List::new(items).block(
            Block::default()
                .title_bottom(Line::from(Span::styled(
                    count_info,
                    Style::default().fg(Color::DarkGray),
                )))
        );
        frame.render_widget(list, chunks[2]);

        // Scrollbar
        if results.len() > result_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None);
            let mut scrollbar_state = ScrollbarState::new(results.len())
                .position(scroll);
            frame.render_stateful_widget(
                scrollbar,
                chunks[2],
                &mut scrollbar_state,
            );
        }
    }

    (result_height, chunks[2])
}

fn fit_to_width(s: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthStr;
    let str_width = UnicodeWidthStr::width(s);
    if str_width <= max_width {
        format!("{}{}", s, " ".repeat(max_width - str_width))
    } else {
        let mut w = 0;
        let mut result = String::new();
        for ch in s.chars() {
            let ch_w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if w + ch_w + 1 > max_width {
                result.push('\u{2026}');
                w += 1;
                break;
            }
            w += ch_w;
            result.push(ch);
        }
        let pad = max_width.saturating_sub(w);
        result.push_str(&" ".repeat(pad));
        result
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
