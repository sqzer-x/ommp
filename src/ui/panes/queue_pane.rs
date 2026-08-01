use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::app::{App, AppAction};
use crate::ui::pane::Pane;
use crate::ui::theme::Theme;

const HOVER_BG: Color = Color::Indexed(238); // very dark gray

pub struct QueuePane {
    pub scroll_offset: usize,
    pub hover_row: Option<usize>,
}

impl QueuePane {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            hover_row: None,
        }
    }
}

/// Color for each audio format extension
fn format_color(ext: &str) -> Color {
    match ext {
        "FLAC" => Color::Green,
        "M4A" | "AAC" | "MP4" | "ALAC" => Color::Cyan,
        "MP3" => Color::Yellow,
        "OGG" => Color::Magenta,
        "WAV" | "WAVE" => Color::Blue,
        _ => Color::White,
    }
}

/// Truncate a string to fit within `max_width` columns, adding "…" if needed.
/// Pads with spaces to exactly fill `max_width`.
fn fit_to_width(s: &str, max_width: usize) -> String {
    let str_width = UnicodeWidthStr::width(s);
    if str_width <= max_width {
        let padding = max_width - str_width;
        format!("{}{}", s, " ".repeat(padding))
    } else {
        let mut w = 0;
        let mut result = String::new();
        for ch in s.chars() {
            let ch_w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if w + ch_w + 1 > max_width {
                result.push('\u{2026}'); // …
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

impl Pane for QueuePane {
    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool, app: &App, theme: &Theme) {
        let count = app.queue.tracks.len();
        let border_color = if focused {
            theme.border_focused
        } else {
            theme.border_unfocused
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title_style(Style::default().fg(if focused {
                theme.border_focused
            } else {
                theme.fg
            }));

        let inner = block.inner(area);
        let inner_height = inner.height as usize;
        let inner_width = inner.width as usize;

        // Auto-scroll to keep selected_index visible
        if count > 0 {
            if app.queue.selected_index < self.scroll_offset {
                self.scroll_offset = app.queue.selected_index;
            }
            if inner_height > 0 && app.queue.selected_index >= self.scroll_offset + inner_height {
                self.scroll_offset = app.queue.selected_index - inner_height + 1;
            }
        }

        // Column layout: prefix(2) + title(55%) + artist(45%) + ext(4) + gap(1) + dur(5) + trail(1)
        let ext_col_width = 4;
        let dur_col_width = 5;
        let prefix_width = 2;
        let fixed_width = prefix_width + 1 + ext_col_width + 1 + dur_col_width + 1;
        let flex_total = inner_width.saturating_sub(fixed_width);
        let title_max = (flex_total * 55 / 100).max(4);
        let artist_max = flex_total.saturating_sub(title_max).max(4);
        let has_scrollbar = count > inner_height;

        let items: Vec<ListItem> = app
            .queue
            .tracks
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(inner_height)
            .map(|(i, &track_idx)| {
                let track = &app.library.tracks[track_idx];
                let is_current = app.queue.current_index == Some(i);
                let is_selected = i == app.queue.selected_index;

                let artist = track.display_artist();
                let ext = track
                    .path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("?")
                    .to_uppercase();
                let dur = track.format_duration();

                // Base styles
                let sel_style = Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .add_modifier(Modifier::BOLD);
                let cur_style = theme.current_track_style;
                let normal_style = Style::default().fg(theme.fg);
                let dim_style = Style::default().fg(Color::Gray);

                let is_hovered = self.hover_row == Some(i);

                let (title_style, artist_style, ext_style, dur_style, prefix_style) =
                    if is_selected && focused {
                        (sel_style, sel_style, sel_style, sel_style, sel_style)
                    } else if is_current {
                        let bg = if is_hovered { HOVER_BG } else { Color::Reset };
                        (
                            cur_style.bg(bg),
                            dim_style.bg(bg),
                            Style::default().fg(format_color(&ext)).add_modifier(Modifier::BOLD).bg(bg),
                            Style::default().fg(Color::DarkGray).bg(bg),
                            cur_style.bg(bg),
                        )
                    } else if is_hovered {
                        (
                            normal_style.bg(HOVER_BG),
                            dim_style.bg(HOVER_BG),
                            Style::default().fg(format_color(&ext)).bg(HOVER_BG),
                            Style::default().fg(Color::DarkGray).bg(HOVER_BG),
                            normal_style.bg(HOVER_BG),
                        )
                    } else {
                        (
                            normal_style,
                            dim_style,
                            Style::default().fg(format_color(&ext)),
                            Style::default().fg(Color::DarkGray),
                            normal_style,
                        )
                    };

                let in_playlist = app.playlists.iter().any(|pl| pl.tracks.contains(&track_idx));
                let prefix = if is_current { "\u{F04B} " } else { "  " }; // nf-fa-play

                // Star integrated into title text so it stays next to the title
                let title_text = if in_playlist {
                    format!("{} \u{F005}", track.title) // "Title nf-fa-star"
                } else {
                    track.title.clone()
                };
                let title_fitted = fit_to_width(&title_text, title_max);
                let artist_fitted = fit_to_width(artist, artist_max);

                // Right-align ext to ext_col_width
                let ext_padded = format!("{:>width$}", ext, width = ext_col_width);
                // Right-align dur to dur_col_width
                let dur_padded = format!("{:>width$}", dur, width = dur_col_width);

                // Row background for gap spans (keeps selection/hover highlight continuous)
                let row_bg = if is_selected && focused {
                    sel_style
                } else if is_hovered {
                    Style::default().bg(HOVER_BG)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, prefix_style),
                    Span::styled(title_fitted, title_style),
                    Span::styled(artist_fitted, artist_style),
                    Span::styled(" ", row_bg),
                    Span::styled(ext_padded, ext_style),
                    Span::styled(" ", row_bg),
                    Span::styled(dur_padded, dur_style),
                    Span::styled(" ", row_bg),
                ]))
            })
            .collect();

        let list = List::new(items).block(block);
        frame.render_widget(list, area);

        // Scrollbar (rendered over the right border)
        if has_scrollbar {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None);
            let mut scrollbar_state = ScrollbarState::new(count)
                .position(self.scroll_offset);
            frame.render_stateful_widget(
                scrollbar,
                area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 }),
                &mut scrollbar_state,
            );
        }
    }

    fn handle_key(&mut self, key: KeyEvent, app: &App) -> Option<AppAction> {
        let count = app.queue.tracks.len();
        if count == 0 && key.code != KeyCode::Char('c') {
            return None;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => None,
            KeyCode::Char('k') | KeyCode::Up => None,
            KeyCode::Enter => {
                if count > 0 {
                    Some(AppAction::PlayQueueIndex(app.queue.selected_index))
                } else {
                    None
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                if count > 0 {
                    Some(AppAction::RemoveFromQueue(app.queue.selected_index))
                } else {
                    None
                }
            }
            KeyCode::Char('c') => Some(AppAction::ClearQueue),
            _ => None,
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect, app: &App) -> Option<AppAction> {
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        let count = app.queue.tracks.len();

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if event.column >= inner.x
                    && event.column < inner.x + inner.width
                    && event.row >= inner.y
                    && event.row < inner.y + inner.height
                {
                    let clicked = self.scroll_offset + (event.row - inner.y) as usize;
                    if clicked < count {
                        // Selection handled by handler.rs
                    }
                }
                None
            }
            MouseEventKind::ScrollDown => self.handle_scroll(false, app),
            MouseEventKind::ScrollUp => self.handle_scroll(true, app),
            _ => None,
        }
    }

    fn handle_scroll(&mut self, up: bool, app: &App) -> Option<AppAction> {
        let count = app.queue.tracks.len();
        if count == 0 {
            return None;
        }
        if up {
            self.scroll_offset = self.scroll_offset.saturating_sub(3);
            let new_sel = app.queue.selected_index.saturating_sub(3);
            Some(AppAction::SetQueueSelection(new_sel))
        } else {
            self.scroll_offset = (self.scroll_offset + 3).min(count.saturating_sub(1));
            let new_sel = (app.queue.selected_index + 3).min(count.saturating_sub(1));
            Some(AppAction::SetQueueSelection(new_sel))
        }
    }
}
