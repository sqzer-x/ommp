pub mod layout;
pub mod pane;
pub mod panes;
pub mod theme;
pub mod widgets;

use ratatui::Frame;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};
use widgets::{about_modal, help_modal, playlist_modal, search_modal};
use widgets::playlist_modal::PlaylistModalMode;

use crate::app::App;
use crate::app::state::{FocusedPane, InfoView, Tab};
use layout::LayoutAreas;
use pane::Pane;
use panes::albums_pane::AlbumsPane;
use panes::artists_pane::ArtistsPane;
use panes::dir_browser_pane::DirBrowserPane;
use panes::format_pane::FormatPane;
use panes::genre_pane::GenrePane;
use panes::library_pane::LibraryPane;
use panes::lyrics_pane::LyricsPane;
use panes::playlists_pane::PlaylistsPane;
use panes::queue_pane::QueuePane;
use theme::Theme;
use widgets::info_pane;
use widgets::progress_bar;
use widgets::status_bar;
use widgets::tab_bar;

pub struct Ui {
    pub theme: Theme,
    pub library_pane: LibraryPane,
    pub dir_browser_pane: DirBrowserPane,
    pub queue_pane: QueuePane,
    pub artists_pane: ArtistsPane,
    pub albums_pane: AlbumsPane,
    pub genre_pane: GenrePane,
    pub format_pane: FormatPane,
    pub playlists_pane: PlaylistsPane,
    pub lyrics_pane: LyricsPane,
    pub last_click: Option<(std::time::Instant, u16, u16)>,
    /// Last known mouse position (column, row) for hover tracking
    pub mouse_pos: Option<(u16, u16)>,
    /// Tab index currently hovered by mouse
    pub hovered_tab: Option<usize>,
    /// Pane width percentages [Library, Playlist, Lyrics], sum = 100
    pub pane_widths: [u16; 3],
    /// Resize mode active (Ctrl+E)
    pub resize_mode: bool,
    /// Border being dragged: 0 = lib|playlist, 1 = playlist|lyrics, 2 = info|lyrics (horizontal), None = not dragging
    pub dragging_border: Option<u8>,
    /// Right column split: info pane height percentage (top), lyrics gets the rest
    pub right_split: u16,
    /// Ctrl+E pressed, waiting for next key
    pub chord_pending: bool,
    /// Help modal visible
    pub show_help_modal: bool,
    /// Search modal visible
    pub show_search_modal: bool,
    /// Search modal input text
    pub search_modal_input: String,
    /// Search modal filtered results (track indices)
    pub search_modal_results: Vec<usize>,
    /// Search modal selected result index
    pub search_modal_selected: usize,
    /// Search modal scroll offset
    pub search_modal_scroll: usize,
    /// Search modal visible result row count (set during render)
    pub search_modal_result_height: usize,
    /// Search modal result area rect (set during render, for mouse hit-testing)
    pub search_modal_result_area: ratatui::layout::Rect,
    /// Search modal hovered row index
    pub search_modal_hover_row: Option<usize>,
    /// Playlist modal visible ("b" key)
    pub show_playlist_modal: bool,
    /// Playlist modal selected index
    pub playlist_modal_selected: usize,
    /// Playlist modal mode (List / Create / Rename)
    pub playlist_modal_mode: PlaylistModalMode,
    /// Playlist modal text input (for create/rename)
    pub playlist_modal_input: String,
    /// About modal visible
    pub show_about_modal: bool,
    /// Splash screen visible at startup
    pub show_splash: bool,
    /// Splash screen start time
    pub splash_start: Option<std::time::Instant>,
    /// Current info pane view (Clock / AlbumArt / TrackInfo)
    pub info_view: InfoView,
    /// Album art pixel cache
    pub album_art_cache: info_pane::AlbumArtCache,
}

impl Ui {
    pub fn new(music_dir: std::path::PathBuf, picker: ratatui_image::picker::Picker) -> Self {
        Self {
            theme: Theme::default(),
            library_pane: LibraryPane::new(),
            dir_browser_pane: DirBrowserPane::new(music_dir),
            queue_pane: QueuePane::new(),
            artists_pane: ArtistsPane::new(),
            albums_pane: AlbumsPane::new(),
            genre_pane: GenrePane::new(),
            format_pane: FormatPane::new(),
            playlists_pane: PlaylistsPane::new(),
            lyrics_pane: LyricsPane::new(),
            last_click: None,
            mouse_pos: None,
            hovered_tab: None,
            pane_widths: [20, 60, 20],
            resize_mode: false,
            dragging_border: None,
            right_split: 50,
            chord_pending: false,
            show_help_modal: false,
            show_search_modal: false,
            search_modal_input: String::new(),
            search_modal_results: Vec::new(),
            search_modal_selected: 0,
            search_modal_scroll: 0,
            search_modal_result_height: 10,
            search_modal_result_area: ratatui::layout::Rect::default(),
            search_modal_hover_row: None,
            show_playlist_modal: false,
            playlist_modal_selected: 0,
            playlist_modal_mode: PlaylistModalMode::List,
            playlist_modal_input: String::new(),
            show_about_modal: false,
            show_splash: true,
            splash_start: Some(std::time::Instant::now()),
            info_view: InfoView::Clock,
            album_art_cache: info_pane::AlbumArtCache::new(picker),
        }
    }

    pub fn render(&mut self, frame: &mut Frame, app: &App) {
        if self.show_splash {
            // Timeline: 0–0.5s fade-in, 0.5–1.5s hold, 1.5–2.0s fade-out
            let elapsed = self.splash_start
                .map(|s| s.elapsed().as_secs_f32())
                .unwrap_or(2.0);
            let opacity = if elapsed < 0.5 {
                elapsed / 0.5
            } else if elapsed < 1.5 {
                1.0
            } else {
                (1.0 - (elapsed - 1.5) / 0.5).max(0.0)
            };
            about_modal::render_splash_screen(frame, frame.area(), &self.theme, opacity);
            return;
        }

        let areas = LayoutAreas::compute(frame.area(), self.pane_widths, self.right_split);

        // Status bar
        status_bar::render_status_bar(frame, areas.status_bar, app, &self.theme, self.resize_mode);

        // Tab bar
        tab_bar::render_tab_bar(frame, areas.tab_bar, app.tab, self.hovered_tab, &self.theme);

        // Left pane (varies by tab)
        let lib_focused = app.focus == FocusedPane::Library;
        match app.tab {
            Tab::Queue => self.library_pane.render(frame, areas.library, lib_focused, app, &self.theme),
            Tab::Directories => self.dir_browser_pane.render(frame, areas.library, lib_focused, app, &self.theme),
            Tab::Artists => self.artists_pane.render(frame, areas.library, lib_focused, app, &self.theme),
            Tab::Albums => self.albums_pane.render(frame, areas.library, lib_focused, app, &self.theme),
            Tab::Genre => self.genre_pane.render(frame, areas.library, lib_focused, app, &self.theme),
            Tab::Format => self.format_pane.render(frame, areas.library, lib_focused, app, &self.theme),
            Tab::Playlists => self.playlists_pane.render(frame, areas.library, lib_focused, app, &self.theme),
        }

        // Center pane (Queue)
        let playlist_focused = app.focus == FocusedPane::Playlist;
        self.queue_pane.render(frame, areas.playlist, playlist_focused, app, &self.theme);

        // Right pane top (Info)
        info_pane::render_info_pane(frame, areas.info_pane, app, &self.theme, self.info_view, &mut self.album_art_cache);

        // Right pane bottom (Lyrics)
        let lyrics_focused = app.focus == FocusedPane::Lyrics;
        self.lyrics_pane.render(frame, areas.lyrics, lyrics_focused, app, &self.theme);

        // Progress bar
        progress_bar::render_progress_bar(frame, areas.progress_bar, app, &self.theme);

        // Resize mode: overlay yellow border on focused pane
        if self.resize_mode {
            let focused_area = match app.focus {
                FocusedPane::Library => areas.library,
                FocusedPane::Playlist => areas.playlist,
                FocusedPane::Lyrics => areas.lyrics,
            };
            let overlay = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            frame.render_widget(overlay, focused_area);
        }

        // Modal overlays (rendered last, on top of everything)
        if self.show_search_modal {
            let (rh, ra) = search_modal::render_search_modal(
                frame,
                frame.area(),
                &self.search_modal_input,
                &self.search_modal_results,
                self.search_modal_selected,
                self.search_modal_scroll,
                self.search_modal_hover_row,
                app,
                &self.theme,
            );
            self.search_modal_result_height = rh;
            self.search_modal_result_area = ra;
        }

        if self.show_help_modal {
            help_modal::render_help_modal(frame, frame.area(), &self.theme);
        }

        if self.show_about_modal {
            about_modal::render_about_modal(frame, frame.area(), &self.theme);
        }

        if self.show_playlist_modal {
            playlist_modal::render_playlist_modal(
                frame,
                frame.area(),
                self.playlist_modal_selected,
                &self.playlist_modal_mode,
                &self.playlist_modal_input,
                app,
                &self.theme,
            );
        }
    }

    pub fn refresh_dir_browser(&mut self, app: &App) {
        self.dir_browser_pane.refresh(app);
    }

    pub fn clamp_selections(&mut self, app: &App) {
        let artists_len = app.library.get_artists().len();
        if artists_len == 0 {
            self.artists_pane.selected = 0;
            self.artists_pane.scroll_offset = 0;
        } else {
            self.artists_pane.selected = self.artists_pane.selected.min(artists_len - 1);
            self.artists_pane.scroll_offset = self.artists_pane.scroll_offset.min(artists_len - 1);
        }

        let albums_len = app.library.get_albums().len();
        if albums_len == 0 {
            self.albums_pane.selected = 0;
            self.albums_pane.scroll_offset = 0;
        } else {
            self.albums_pane.selected = self.albums_pane.selected.min(albums_len - 1);
            self.albums_pane.scroll_offset = self.albums_pane.scroll_offset.min(albums_len - 1);
        }

        let genres_len = app.library.get_genres().len();
        if genres_len == 0 {
            self.genre_pane.selected = 0;
            self.genre_pane.scroll_offset = 0;
        } else {
            self.genre_pane.selected = self.genre_pane.selected.min(genres_len - 1);
            self.genre_pane.scroll_offset = self.genre_pane.scroll_offset.min(genres_len - 1);
        }

        let formats_len = app.library.get_formats().len();
        if formats_len == 0 {
            self.format_pane.selected = 0;
            self.format_pane.scroll_offset = 0;
        } else {
            self.format_pane.selected = self.format_pane.selected.min(formats_len - 1);
            self.format_pane.scroll_offset = self.format_pane.scroll_offset.min(formats_len - 1);
        }

        let playlists_len = app.playlists.len();
        if playlists_len == 0 {
            self.playlists_pane.selected = 0;
            self.playlists_pane.scroll_offset = 0;
        } else {
            self.playlists_pane.selected = self.playlists_pane.selected.min(playlists_len - 1);
            self.playlists_pane.scroll_offset = self.playlists_pane.scroll_offset.min(playlists_len - 1);
        }

        // Reset library/dir browser to top since track indices changed
        self.library_pane.selected = 0;
        self.library_pane.scroll_offset = 0;
        self.dir_browser_pane.selected = 0;
        self.dir_browser_pane.scroll_offset = 0;

        // Clamp queue pane scroll
        let queue_len = app.queue.tracks.len();
        if queue_len == 0 {
            self.queue_pane.scroll_offset = 0;
        } else {
            self.queue_pane.scroll_offset = self.queue_pane.scroll_offset.min(queue_len - 1);
        }
    }
}
