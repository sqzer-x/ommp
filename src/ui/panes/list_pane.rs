//! One scrollable list of labelled rows, shared by the Artists, Albums, Genre,
//! Format and Playlists tabs.
//!
//! These were five near-identical files; normalising identifiers made three of
//! them byte-for-byte the same over all 185 lines, and the other two differed
//! only inside the row-render closure and one call in the Enter arm. Everything
//! here beyond `ListRow` and `rows_for`/`activate` was copied five ways, which
//! is how the render-time selection clamp ended up in exactly one of them.

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;

use crate::app::state::Tab;
use crate::app::{App, AppAction};
use crate::ui::text::{fit_to_width, max_scroll};
use crate::ui::theme::Theme;

const HOVER_BG: Color = Color::Indexed(238);
const SCROLL_STEP: usize = 3;

/// A row: an optional leading icon, the label, and dim trailing detail such as
/// an album's artist or a track count.
pub struct ListRow {
    pub icon: Option<(&'static str, Color)>,
    pub text: String,
    pub detail: Option<String>,
}

impl ListRow {
    pub fn plain(text: String) -> Self {
        Self { icon: None, text, detail: None }
    }

    pub fn with_detail(text: String, detail: String) -> Self {
        Self { icon: None, text, detail: Some(detail) }
    }
}

#[derive(Default)]
pub struct ListPane {
    pub selected: usize,
    pub scroll_offset: usize,
    pub hover_row: Option<usize>,
}

impl ListPane {
    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        focused: bool,
        theme: &Theme,
        rows: &[ListRow],
    ) {
        let count = rows.len();
        // Clamp here rather than trusting callers: only one of the five copies
        // did this, and it survived purely because a separate clamp ran on
        // library reload.
        if self.selected >= count {
            self.selected = count.saturating_sub(1);
        }

        let border_color = if focused {
            theme.border_focused
        } else {
            theme.border_unfocused
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        let inner_height = inner.height as usize;
        let inner_width = inner.width as usize;

        // Auto-scroll to keep the selection visible, then make sure the viewport
        // is still full.
        if count > 0 && inner_height > 0 {
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
            if self.selected >= self.scroll_offset + inner_height {
                self.scroll_offset = self.selected - inner_height + 1;
            }
            self.scroll_offset = self.scroll_offset.min(max_scroll(count, inner_height));
        }

        let highlight = Style::default()
            .bg(theme.highlight_bg)
            .fg(theme.highlight_fg)
            .add_modifier(Modifier::BOLD);

        let items: Vec<ListItem> = rows
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(inner_height)
            .map(|(i, row)| {
                let selected = i == self.selected && focused;
                let hovered = self.hover_row == Some(i);
                let bg = if hovered { HOVER_BG } else { Color::Reset };

                let base = if selected {
                    highlight
                } else {
                    Style::default().fg(theme.fg).bg(bg)
                };
                let dim = if selected {
                    highlight
                } else {
                    Style::default().fg(Color::DarkGray).bg(bg)
                };

                let mut spans = Vec::new();
                let mut used = 2;
                spans.push(Span::styled("  ", base));
                if let Some((icon, color)) = row.icon {
                    used += icon.chars().count() + 1;
                    let style = if selected {
                        highlight
                    } else {
                        Style::default().fg(color).bg(bg)
                    };
                    spans.push(Span::styled(format!("{icon} "), style));
                }

                match &row.detail {
                    // Pad the label to a fixed column so the detail lines up on
                    // every row. Without this a CJK label and an ASCII one put
                    // the second column at different offsets.
                    Some(detail) => {
                        let detail_w = detail.chars().count() + 1;
                        let label_w = inner_width.saturating_sub(used + detail_w);
                        spans.push(Span::styled(fit_to_width(&row.text, label_w), base));
                        spans.push(Span::styled(format!(" {detail}"), dim));
                    }
                    None => spans.push(Span::styled(
                        fit_to_width(&row.text, inner_width.saturating_sub(used)),
                        base,
                    )),
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        frame.render_widget(List::new(items).block(block), area);

        if count > inner_height {
            let mut state = ScrollbarState::new(count).position(self.scroll_offset);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None),
                area.inner(Margin { vertical: 1, horizontal: 0 }),
                &mut state,
            );
        }
    }

    /// Returns true when the key selected an item and the caller should build an
    /// activation action.
    pub fn handle_key(&mut self, key: KeyEvent, count: usize) -> bool {
        if count == 0 {
            return false;
        }
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.selected < count - 1 {
                    self.selected += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.selected = 0;
                self.scroll_offset = 0;
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.selected = count - 1;
            }
            KeyCode::Enter => return true,
            _ => {}
        }
        false
    }

    pub fn handle_mouse(&mut self, event: MouseEvent, area: Rect, count: usize) {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let inner = Block::default().borders(Borders::ALL).inner(area);
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
            }
            MouseEventKind::ScrollDown => self.scroll(false, count, area.height as usize),
            MouseEventKind::ScrollUp => self.scroll(true, count, area.height as usize),
            _ => {}
        }
    }

    pub fn scroll(&mut self, up: bool, count: usize, viewport: usize) {
        if count == 0 {
            return;
        }
        // Two border rows.
        let inner_height = viewport.saturating_sub(2);
        if up {
            self.scroll_offset = self.scroll_offset.saturating_sub(SCROLL_STEP);
            self.selected = self.selected.saturating_sub(SCROLL_STEP);
        } else {
            self.scroll_offset =
                (self.scroll_offset + SCROLL_STEP).min(max_scroll(count, inner_height));
            self.selected = (self.selected + SCROLL_STEP).min(count - 1);
        }
    }
}

/// Rows for whichever of the five list tabs is showing.
pub fn rows_for(app: &App, tab: Tab) -> Vec<ListRow> {
    match tab {
        Tab::Artists => app.library.get_artists().into_iter().map(ListRow::plain).collect(),
        Tab::Albums => app
            .library
            .get_albums()
            .into_iter()
            .map(|(album, artist)| ListRow::with_detail(album, artist))
            .collect(),
        Tab::Genre => app.library.get_genres().into_iter().map(ListRow::plain).collect(),
        Tab::Format => {
            // One pass over the library instead of one per visible row: the old
            // format pane called get_tracks_by_format inside the render closure,
            // rescanning and lowercasing every track for every row, every frame.
            let counts = app.library.format_counts();
            counts
                .into_iter()
                .map(|(fmt, n)| ListRow::with_detail(fmt, format!("({n})")))
                .collect()
        }
        Tab::Playlists => app
            .playlists
            .iter()
            .map(|pl| ListRow {
                icon: Some(("\u{F005}", Color::Yellow)),
                text: pl.name.clone(),
                detail: Some(format!("({})", pl.tracks.len())),
            })
            .collect(),
        Tab::Queue | Tab::Directories => Vec::new(),
    }
}

/// The action for activating row `selected` of `tab`.
pub fn activate(app: &App, tab: Tab, selected: usize) -> Option<AppAction> {
    let tracks = match tab {
        Tab::Artists => app.library.get_tracks_by_artist(app.library.get_artists().get(selected)?),
        Tab::Albums => {
            let albums = app.library.get_albums();
            let (album, artist) = albums.get(selected)?;
            app.library.get_tracks_by_album(album, artist)
        }
        Tab::Genre => app.library.get_tracks_by_genre(app.library.get_genres().get(selected)?),
        Tab::Format => app.library.get_tracks_by_format(app.library.get_formats().get(selected)?),
        Tab::Playlists => app.playlists.get(selected)?.tracks.clone(),
        Tab::Queue | Tab::Directories => return None,
    };
    (!tracks.is_empty()).then_some(AppAction::AddToQueue(tracks))
}
