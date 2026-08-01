use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use crate::app::{App, AppAction};
use crate::ui::pane::Pane;
use crate::ui::theme::Theme;

const HOVER_BG: Color = Color::Indexed(238);

pub struct GenrePane {
    pub selected: usize,
    pub scroll_offset: usize,
    pub hover_row: Option<usize>,
}

impl GenrePane {
    pub fn new() -> Self {
        Self {
            selected: 0,
            scroll_offset: 0,
            hover_row: None,
        }
    }
}

impl Pane for GenrePane {
    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool, app: &App, theme: &Theme) {
        let genres = app.library.get_genres();
        let count = genres.len();
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

        // Auto-scroll
        if count > 0 {
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
            if inner_height > 0 && self.selected >= self.scroll_offset + inner_height {
                self.scroll_offset = self.selected - inner_height + 1;
            }
        }

        let has_scrollbar = count > inner_height;
        let highlight = Style::default()
            .bg(theme.highlight_bg)
            .fg(theme.highlight_fg)
            .add_modifier(Modifier::BOLD);

        let items: Vec<ListItem> = genres
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(inner_height)
            .map(|(i, genre)| {
                let is_selected = i == self.selected;
                let is_hovered = self.hover_row == Some(i);
                let style = if is_selected && focused {
                    highlight
                } else if is_hovered {
                    Style::default().fg(theme.fg).bg(HOVER_BG)
                } else {
                    Style::default().fg(theme.fg)
                };
                ListItem::new(Line::from(Span::styled(format!("  {}", genre), style)))
            })
            .collect();

        let list = List::new(items).block(block);
        frame.render_widget(list, area);

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
        let genres = app.library.get_genres();
        let count = genres.len();
        if count == 0 {
            return None;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.selected < count - 1 {
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
                if self.selected < count {
                    let tracks = app.library.get_tracks_by_genre(&genres[self.selected]);
                    if !tracks.is_empty() {
                        return Some(AppAction::AddToQueue(tracks));
                    }
                }
                None
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.selected = 0;
                self.scroll_offset = 0;
                None
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.selected = count.saturating_sub(1);
                None
            }
            _ => None,
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect, app: &App) -> Option<AppAction> {
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        let count = app.library.get_genres().len();

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if event.column >= inner.x
                    && event.column < inner.x + inner.width
                    && event.row >= inner.y
                    && event.row < inner.y + inner.height
                {
                    let clicked = self.scroll_offset + (event.row - inner.y) as usize;
                    if clicked < count {
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
        let count = app.library.get_genres().len();
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
