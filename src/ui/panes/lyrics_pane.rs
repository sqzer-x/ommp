use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use crate::app::{App, AppAction};
use crate::ui::pane::Pane;
use crate::ui::theme::Theme;
use crate::ui::widgets::info_pane;

pub struct LyricsPane {
    pub scroll_offset: u16,
}

impl LyricsPane {
    pub fn new() -> Self {
        Self { scroll_offset: 0 }
    }
}

impl Pane for LyricsPane {
    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool, app: &App, theme: &Theme) {
        let border_color = if focused {
            theme.border_focused
        } else {
            theme.border_unfocused
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        info_pane::render_track_info(frame, inner, app, theme);
    }

    fn handle_key(&mut self, key: KeyEvent, _app: &App) -> Option<AppAction> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                None
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.scroll_offset = 0;
                None
            }
            _ => None,
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent, _area: Rect, _app: &App) -> Option<AppAction> {
        match event.kind {
            MouseEventKind::ScrollDown => {
                self.scroll_offset = self.scroll_offset.saturating_add(3);
                None
            }
            MouseEventKind::ScrollUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(3);
                None
            }
            _ => None,
        }
    }

    fn handle_scroll(&mut self, up: bool, _app: &App) -> Option<AppAction> {
        if up {
            self.scroll_offset = self.scroll_offset.saturating_sub(3);
        } else {
            self.scroll_offset = self.scroll_offset.saturating_add(3);
        }
        None
    }
}
