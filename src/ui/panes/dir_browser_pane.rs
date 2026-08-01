use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;
use std::path::PathBuf;

use crate::app::{App, AppAction};
use crate::ui::pane::Pane;
use crate::ui::theme::Theme;

const HOVER_BG: Color = Color::Indexed(238);

pub struct DirBrowserPane {
    pub current_dir: PathBuf,
    pub entries: Vec<DirEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub hover_row: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum DirEntry {
    ParentDir,
    Directory(String),
    Track(usize),
}

impl DirBrowserPane {
    pub fn new(music_dir: PathBuf) -> Self {
        Self {
            current_dir: music_dir,
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            hover_row: None,
        }
    }

    pub fn refresh(&mut self, app: &App) {
        self.entries.clear();

        if self.current_dir != app.music_dir {
            self.entries.push(DirEntry::ParentDir);
        }

        let (subdirs, tracks) = app.library.get_directory_entries(&self.current_dir);
        for dir in subdirs {
            self.entries.push(DirEntry::Directory(dir));
        }
        for track_idx in tracks {
            self.entries.push(DirEntry::Track(track_idx));
        }
    }
}

impl Pane for DirBrowserPane {
    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool, app: &App, theme: &Theme) {
        let count = self.entries.len();
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

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(inner_height)
            .map(|(i, entry)| {
                let is_selected = i == self.selected;
                let is_hovered = self.hover_row == Some(i);

                match entry {
                    DirEntry::ParentDir => {
                        if is_selected && focused {
                            ListItem::new(Line::from(vec![
                                Span::styled("  \u{F07B} ", highlight),
                                Span::styled("..", highlight),
                            ]))
                        } else {
                            let bg = if is_hovered { HOVER_BG } else { Color::Reset };
                            ListItem::new(Line::from(vec![
                                Span::styled("  \u{F07B} ", Style::default().fg(Color::Yellow).bg(bg)),
                                Span::styled("..", Style::default().fg(theme.fg).bg(bg)),
                            ]))
                        }
                    }
                    DirEntry::Directory(name) => {
                        if is_selected && focused {
                            ListItem::new(Line::from(vec![
                                Span::styled("  \u{F07B} ", highlight),
                                Span::styled(format!("{}/", name), highlight),
                            ]))
                        } else {
                            let bg = if is_hovered { HOVER_BG } else { Color::Reset };
                            ListItem::new(Line::from(vec![
                                Span::styled("  \u{F07B} ", Style::default().fg(Color::Green).bg(bg)),
                                Span::styled(format!("{}/", name), Style::default().fg(theme.fg).bg(bg)),
                            ]))
                        }
                    }
                    DirEntry::Track(idx) => {
                        let t = &app.library.tracks[*idx];
                        if is_selected && focused {
                            ListItem::new(Line::from(vec![
                                Span::styled("  \u{F001} ", highlight),
                                Span::styled(&t.title, highlight),
                            ]))
                        } else {
                            let bg = if is_hovered { HOVER_BG } else { Color::Reset };
                            ListItem::new(Line::from(vec![
                                Span::styled("  \u{F001} ", Style::default().fg(Color::Cyan).bg(bg)),
                                Span::styled(&t.title, Style::default().fg(theme.fg).bg(bg)),
                            ]))
                        }
                    }
                }
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
        let count = self.entries.len();
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
                match &self.entries[self.selected] {
                    DirEntry::ParentDir => {
                        if let Some(parent) = self.current_dir.parent() {
                            self.current_dir = parent.to_path_buf();
                            self.selected = 0;
                            self.scroll_offset = 0;
                            self.refresh(app);
                        }
                    }
                    DirEntry::Directory(name) => {
                        self.current_dir = self.current_dir.join(name);
                        self.selected = 0;
                        self.scroll_offset = 0;
                        self.refresh(app);
                    }
                    DirEntry::Track(idx) => {
                        return Some(AppAction::AddToQueue(vec![*idx]));
                    }
                }
                None
            }
            KeyCode::Backspace => {
                if let Some(parent) = self.current_dir.parent() {
                    self.current_dir = parent.to_path_buf();
                    self.selected = 0;
                    self.scroll_offset = 0;
                    self.refresh(app);
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

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect, _app: &App) -> Option<AppAction> {
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        let count = self.entries.len();

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
            MouseEventKind::ScrollDown => self.handle_scroll(false, _app),
            MouseEventKind::ScrollUp => self.handle_scroll(true, _app),
            _ => None,
        }
    }

    fn handle_scroll(&mut self, up: bool, _app: &App) -> Option<AppAction> {
        let count = self.entries.len();
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
