use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, AppAction};
use crate::ui::pane::Pane;
use crate::ui::theme::Theme;

pub struct SearchPane {
    pub selected: usize,
    pub scroll_offset: usize,
    pub input_mode: bool,
    pub hover_row: Option<usize>,
}

impl SearchPane {
    pub fn new() -> Self {
        Self {
            selected: 0,
            scroll_offset: 0,
            input_mode: false,
            hover_row: None,
        }
    }
}

impl Pane for SearchPane {
    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool, app: &App, theme: &Theme) {
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
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(inner);

        // Search input line
        let input_style = if self.input_mode {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Gray)
        };

        let cursor = if self.input_mode { "▎" } else { "" };
        let input = Paragraph::new(Line::from(vec![
            Span::styled(" / ", Style::default().fg(Color::Cyan)),
            Span::styled(&app.search_query, input_style),
            Span::styled(cursor, Style::default().fg(Color::White)),
        ]));
        frame.render_widget(input, chunks[0]);

        // Results
        let inner_height = chunks[1].height as usize;
        let items: Vec<ListItem> = app
            .search_results
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(inner_height)
            .map(|(i, &track_idx)| {
                let track = &app.library.tracks[track_idx];
                let is_selected = i == self.selected;
                let style = if is_selected && focused {
                    Style::default()
                        .bg(theme.highlight_bg)
                        .fg(theme.highlight_fg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                };

                let artist_style = if is_selected && focused {
                    style
                } else {
                    Style::default().fg(Color::Gray)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(&track.title, style),
                    Span::styled(format!(" - {}", track.display_artist()), artist_style),
                ]))
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, chunks[1]);
    }

    fn handle_key(&mut self, key: KeyEvent, app: &App) -> Option<AppAction> {
        if self.input_mode || app.search_mode {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = false;
                    return Some(AppAction::ExitSearchMode);
                }
                KeyCode::Enter => {
                    self.input_mode = false;
                    // Add selected search result to queue
                    if self.selected < app.search_results.len() {
                        let track_idx = app.search_results[self.selected];
                        return Some(AppAction::AddToQueue(vec![track_idx]));
                    }
                    return Some(AppAction::ExitSearchMode);
                }
                KeyCode::Backspace => {
                    let mut q = app.search_query.clone();
                    q.pop();
                    return Some(AppAction::SearchQuery(q));
                }
                KeyCode::Char(c) => {
                    let mut q = app.search_query.clone();
                    q.push(c);
                    return Some(AppAction::SearchQuery(q));
                }
                _ => {}
            }
            return None;
        }

        match key.code {
            KeyCode::Char('/') | KeyCode::Char('i') => {
                self.input_mode = true;
                Some(AppAction::EnterSearchMode)
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.selected < app.search_results.len().saturating_sub(1) {
                    self.selected += 1;
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                None
            }
            KeyCode::Enter => {
                if self.selected < app.search_results.len() {
                    let track_idx = app.search_results[self.selected];
                    Some(AppAction::AddToQueue(vec![track_idx]))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect, app: &App) -> Option<AppAction> {
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if event.row == inner.y {
                    // Click on search input
                    self.input_mode = true;
                    return Some(AppAction::EnterSearchMode);
                }
                let results_y = inner.y + 1;
                if event.row >= results_y && event.row < inner.y + inner.height {
                    let clicked = self.scroll_offset + (event.row - results_y) as usize;
                    if clicked < app.search_results.len() {
                        self.selected = clicked;
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
        let count = app.search_results.len();
        if count == 0 {
            return None;
        }
        if up {
            self.scroll_offset = self.scroll_offset.saturating_sub(3);
            self.selected = self.selected.saturating_sub(3);
        } else {
            self.scroll_offset = (self.scroll_offset + 3).min(count.saturating_sub(1));
            self.selected = (self.selected + 3).min(count.saturating_sub(1));
        }
        None
    }
}
