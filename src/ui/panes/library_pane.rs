use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use crate::app::{App, AppAction};
use crate::ui::pane::Pane;
use crate::ui::theme::Theme;

/// An entry in the flattened library list
#[derive(Debug, Clone)]
enum LibraryEntry {
    SectionHeader(String),
    Separator,
    AllTracks(usize),
    PlaylistEntry { idx: usize, name: String, count: usize },
    FavoriteDir(String),
    Album { name: String, artist: String },
}

const HOVER_BG: Color = Color::Indexed(238);

/// Library browser for the Queue tab.
/// Shows 4 sections: Playlist, Directories, Albums.
pub struct LibraryPane {
    pub selected: usize,
    pub scroll_offset: usize,
    pub hover_row: Option<usize>,
}

impl LibraryPane {
    pub fn new() -> Self {
        Self {
            selected: 0,
            scroll_offset: 0,
            hover_row: None,
        }
    }

    fn build_entries(app: &App) -> Vec<LibraryEntry> {
        let mut entries = Vec::new();

        // --- Playlist ---
        entries.push(LibraryEntry::SectionHeader("\u{F054} Playlist".into()));
        entries.push(LibraryEntry::AllTracks(app.library.tracks.len()));
        for (idx, pl) in app.playlists.iter().enumerate() {
            entries.push(LibraryEntry::PlaylistEntry {
                idx,
                name: pl.name.clone(),
                count: pl.tracks.len(),
            });
        }

        entries.push(LibraryEntry::Separator);

        // --- Directories ---
        let mut dirs = std::collections::BTreeSet::new();
        for t in &app.library.tracks {
            if let Some(parent) = t.path.parent() {
                if let Some(name) = parent.file_name().and_then(|n| n.to_str()) {
                    dirs.insert(name.to_string());
                }
            }
        }
        entries.push(LibraryEntry::SectionHeader(format!(
            "\u{F054} Directories ({})",
            dirs.len()
        )));
        for d in &dirs {
            entries.push(LibraryEntry::FavoriteDir(d.clone()));
        }

        entries.push(LibraryEntry::Separator);

        // --- Albums ---
        let albums = app.library.get_albums();
        entries.push(LibraryEntry::SectionHeader(format!(
            "\u{F054} Albums ({})",
            albums.len()
        )));
        for (album, artist) in albums {
            entries.push(LibraryEntry::Album { name: album, artist });
        }

        entries
    }
}

impl Pane for LibraryPane {
    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool, app: &App, theme: &Theme) {
        let entries = Self::build_entries(app);
        let count = entries.len();
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

        let items: Vec<ListItem> = entries
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(inner_height)
            .map(|(i, entry)| {
                let is_selected = i == self.selected;
                let is_hovered = self.hover_row == Some(i);
                let highlight = Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .add_modifier(Modifier::BOLD);
                let hover_bg = if is_hovered && !(is_selected && focused) {
                    HOVER_BG
                } else {
                    Color::Reset
                };

                match entry {
                    LibraryEntry::SectionHeader(text) => {
                        let style = if is_selected && focused {
                            highlight
                        } else {
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD)
                                .bg(hover_bg)
                        };
                        ListItem::new(Line::from(Span::styled(text.as_str(), style)))
                    }
                    LibraryEntry::Separator => {
                        ListItem::new(Line::from(""))
                    }
                    LibraryEntry::AllTracks(track_count) => {
                        if is_selected && focused {
                            ListItem::new(Line::from(vec![
                                Span::styled("  \u{F001} ", highlight),
                                Span::styled("All Tracks", highlight),
                                Span::styled(format!(" ({})", track_count), highlight),
                            ]))
                        } else {
                            ListItem::new(Line::from(vec![
                                Span::styled("  \u{F001} ", Style::default().fg(Color::Cyan).bg(hover_bg)),
                                Span::styled("All Tracks", Style::default().fg(theme.fg).bg(hover_bg)),
                                Span::styled(
                                    format!(" ({})", track_count),
                                    Style::default().fg(Color::DarkGray).bg(hover_bg),
                                ),
                            ]))
                        }
                    }
                    LibraryEntry::PlaylistEntry { name, count, .. } => {
                        let icon = "\u{F005} "; // ★
                        if is_selected && focused {
                            ListItem::new(Line::from(vec![
                                Span::styled(format!("  {}", icon), highlight),
                                Span::styled(name.as_str(), highlight),
                                Span::styled(format!(" ({})", count), highlight),
                            ]))
                        } else {
                            ListItem::new(Line::from(vec![
                                Span::styled(format!("  {}", icon), Style::default().fg(Color::Yellow).bg(hover_bg)),
                                Span::styled(name.as_str(), Style::default().fg(theme.fg).bg(hover_bg)),
                                Span::styled(
                                    format!(" ({})", count),
                                    Style::default().fg(Color::DarkGray).bg(hover_bg),
                                ),
                            ]))
                        }
                    }
                    LibraryEntry::FavoriteDir(name) => {
                        if is_selected && focused {
                            ListItem::new(Line::from(vec![
                                Span::styled("  \u{F07B} ", highlight),
                                Span::styled(format!("{}/", name), highlight),
                            ]))
                        } else {
                            ListItem::new(Line::from(vec![
                                Span::styled("  \u{F07B} ", Style::default().fg(Color::Green).bg(hover_bg)),
                                Span::styled(format!("{}/", name), Style::default().fg(theme.fg).bg(hover_bg)),
                            ]))
                        }
                    }
                    LibraryEntry::Album { name, artist } => {
                        let album_display = if name.is_empty() {
                            "Unknown Album"
                        } else {
                            name.as_str()
                        };
                        let artist_display = if artist.is_empty() {
                            ""
                        } else {
                            artist.as_str()
                        };

                        if is_selected && focused {
                            let mut spans = vec![
                                Span::styled("  \u{F0025} ", highlight),
                                Span::styled(album_display, highlight),
                            ];
                            if !artist_display.is_empty() {
                                spans.push(Span::styled(
                                    format!("  {}", artist_display),
                                    highlight,
                                ));
                            }
                            ListItem::new(Line::from(spans))
                        } else {
                            let mut spans = vec![
                                Span::styled(
                                    "  \u{F0025} ",
                                    Style::default().fg(Color::Magenta).bg(hover_bg),
                                ),
                                Span::styled(album_display, Style::default().fg(theme.fg).bg(hover_bg)),
                            ];
                            if !artist_display.is_empty() {
                                spans.push(Span::styled(
                                    format!("  {}", artist_display),
                                    Style::default().fg(Color::Gray).bg(hover_bg),
                                ));
                            }
                            ListItem::new(Line::from(spans))
                        }
                    }
                }
            })
            .collect();

        let list = List::new(items).block(block);
        frame.render_widget(list, area);

        // Scrollbar
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
        let entries = Self::build_entries(app);
        let count = entries.len();
        if count == 0 {
            return None;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.selected < count - 1 {
                    self.selected += 1;
                    // Skip separators when navigating
                    if matches!(entries.get(self.selected), Some(LibraryEntry::Separator))
                        && self.selected < count - 1
                    {
                        self.selected += 1;
                    }
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                    // Skip separators when navigating
                    if matches!(entries.get(self.selected), Some(LibraryEntry::Separator))
                        && self.selected > 0
                    {
                        self.selected -= 1;
                    }
                }
                None
            }
            KeyCode::Enter => {
                if self.selected >= count {
                    return None;
                }
                match &entries[self.selected] {
                    LibraryEntry::SectionHeader(_) | LibraryEntry::Separator => None,
                    LibraryEntry::PlaylistEntry { idx, .. } => {
                        if let Some(pl) = app.playlists.get(*idx) {
                            if !pl.tracks.is_empty() {
                                return Some(AppAction::AddToQueue(pl.tracks.clone()));
                            }
                        }
                        None
                    }
                    LibraryEntry::AllTracks(_) => {
                        let indices: Vec<usize> = (0..app.library.tracks.len()).collect();
                        if !indices.is_empty() {
                            Some(AppAction::AddToQueue(indices))
                        } else {
                            None
                        }
                    }
                    LibraryEntry::FavoriteDir(dir_name) => {
                        let indices: Vec<usize> = app
                            .library
                            .tracks
                            .iter()
                            .enumerate()
                            .filter(|(_, t)| {
                                t.path
                                    .parent()
                                    .and_then(|p| p.file_name())
                                    .and_then(|n| n.to_str())
                                    == Some(dir_name.as_str())
                            })
                            .map(|(i, _)| i)
                            .collect();
                        if !indices.is_empty() {
                            Some(AppAction::AddToQueue(indices))
                        } else {
                            None
                        }
                    }
                    LibraryEntry::Album { name, .. } => {
                        let tracks = app.library.get_tracks_by_album(name);
                        if !tracks.is_empty() {
                            Some(AppAction::AddToQueue(tracks))
                        } else {
                            None
                        }
                    }
                }
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
        let count = Self::build_entries(app).len();

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
        let count = Self::build_entries(app).len();
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
