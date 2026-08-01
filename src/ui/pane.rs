use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::app::{App, AppAction};
use crate::ui::theme::Theme;

pub trait Pane {
    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool, app: &App, theme: &Theme);
    fn handle_key(&mut self, key: KeyEvent, app: &App) -> Option<AppAction>;
    fn handle_mouse(&mut self, event: MouseEvent, area: Rect, app: &App) -> Option<AppAction>;
    fn handle_scroll(&mut self, _up: bool, _app: &App) -> Option<AppAction> {
        None
    }
}
