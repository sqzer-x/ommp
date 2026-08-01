pub mod layout;
pub mod pane;
pub mod panes;
pub mod text;
pub mod theme;
pub mod widgets;

use ratatui::Frame;
use widgets::{about_modal, help_modal, playlist_modal, search_modal};
use widgets::playlist_modal::PlaylistModalMode;

use crate::app::App;
use crate::app::state::{FocusedPane, InfoView, Tab};
use layout::LayoutAreas;
use pane::Pane;
use panes::dir_browser_pane::DirBrowserPane;
use panes::library_pane::LibraryPane;
use panes::list_pane::{self, ListPane};
use panes::track_info_pane::TrackInfoPane;
use panes::queue_pane::QueuePane;

/// Splash timeline, in seconds: fade in, hold, then fade out until the end.
/// Always played in full — the scan runs behind it and does not cut it short.
pub const SPLASH_FADE_IN: f32 = 0.5;
pub const SPLASH_FADE_OUT_AT: f32 = 1.5;
pub const SPLASH_END: f32 = 2.0;

/// Index into `Ui::list_panes` for each list tab. Queue and Directories have
/// their own pane types and never reach here.
pub fn list_slot(tab: Tab) -> usize {
    match tab {
        Tab::Artists => 0,
        Tab::Albums => 1,
        Tab::Genre => 2,
        Tab::Format => 3,
        _ => 4, // Playlists
    }
}
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
    /// One per list tab, keyed by Tab; each keeps its own selection and scroll.
    pub list_panes: [ListPane; 5],
    pub track_info_pane: TrackInfoPane,
    pub last_click: Option<(std::time::Instant, u16, u16)>,
    /// Last known mouse position (column, row) for hover tracking
    pub mouse_pos: Option<(u16, u16)>,
    /// Tab index currently hovered by mouse
    pub hovered_tab: Option<usize>,
    /// Pane width percentages [Library, Playlist, Lyrics], sum = 100
    pub pane_widths: [u16; 3],
    /// Border being dragged: 0 = lib|playlist, 1 = playlist|lyrics, 2 = info|lyrics (horizontal), None = not dragging
    pub dragging_border: Option<u8>,
    /// Right column split: info pane height percentage (top), lyrics gets the rest
    pub right_split: u16,
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
            list_panes: Default::default(),
            track_info_pane: TrackInfoPane::new(),
            last_click: None,
            mouse_pos: None,
            hovered_tab: None,
            pane_widths: [20, 60, 20],
            dragging_border: None,
            right_split: 50,
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

    /// Skip the splash forward to the start of its fade-out, if it has not got
    /// there already. Only a keypress does this: the timeline is otherwise
    /// fixed, and the fade-out still plays so the skip is not a hard cut.
    pub fn begin_splash_fade_out(&mut self) {
        if !self.show_splash {
            return;
        }
        let Some(start) = self.splash_start else { return };
        if start.elapsed().as_secs_f32() >= SPLASH_FADE_OUT_AT {
            return;
        }
        // checked_sub because Instant is CLOCK_MONOTONIC: started within
        // SPLASH_FADE_OUT_AT of boot, plain `-` panics.
        if let Some(t) = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs_f32(SPLASH_FADE_OUT_AT))
        {
            self.splash_start = Some(t);
        }
    }

    pub fn render(&mut self, frame: &mut Frame, app: &App) {
        if self.show_splash {
            let elapsed = self.splash_start
                .map(|s| s.elapsed().as_secs_f32())
                .unwrap_or(SPLASH_END);
            let opacity = if elapsed < SPLASH_FADE_IN {
                elapsed / SPLASH_FADE_IN
            } else if elapsed < SPLASH_FADE_OUT_AT {
                1.0
            } else {
                (1.0 - (elapsed - SPLASH_FADE_OUT_AT) / (SPLASH_END - SPLASH_FADE_OUT_AT)).max(0.0)
            };
            about_modal::render_splash_screen(frame, frame.area(), &self.theme, opacity);
            return;
        }

        let areas = LayoutAreas::compute(frame.area(), self.pane_widths, self.right_split);

        // Status bar
        status_bar::render_status_bar(frame, areas.status_bar, app, &self.theme);

        // Tab bar
        tab_bar::render_tab_bar(frame, areas.tab_bar, app.tab, self.hovered_tab, &self.theme);

        // Left pane (varies by tab)
        let lib_focused = app.focus == FocusedPane::Library;
        match app.tab {
            Tab::Queue => self.library_pane.render(frame, areas.library, lib_focused, app, &self.theme),
            Tab::Directories => self.dir_browser_pane.render(frame, areas.library, lib_focused, app, &self.theme),
            tab => {
                let rows = list_pane::rows_for(app, tab);
                let theme = &self.theme;
                self.list_panes[list_slot(tab)]
                    .render(frame, areas.library, lib_focused, theme, &rows);
            }
        }

        // Center pane (Queue)
        let playlist_focused = app.focus == FocusedPane::Playlist;
        self.queue_pane.render(frame, areas.playlist, playlist_focused, app, &self.theme);

        // Right pane top (Info)
        info_pane::render_info_pane(frame, areas.info_pane, app, &self.theme, self.info_view, &mut self.album_art_cache);

        // Right pane bottom (Lyrics)
        let lyrics_focused = app.focus == FocusedPane::Lyrics;
        self.track_info_pane.render(frame, areas.lyrics, lyrics_focused, app, &self.theme);

        // Progress bar
        progress_bar::render_progress_bar(frame, areas.progress_bar, app, &self.theme);


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

    /// The list pane backing `tab`. Panics only for Queue/Directories, which
    /// have their own pane types — `list_slot` maps those to Playlists, so
    /// callers must check the tab first.
    pub fn list_pane_mut(&mut self, tab: Tab) -> &mut ListPane {
        &mut self.list_panes[list_slot(tab)]
    }

    /// Scroll offset of whichever pane the library column is currently showing.
    pub fn library_pane_scroll(&self, app: &App) -> usize {
        match app.tab {
            Tab::Queue => self.library_pane.scroll_offset,
            Tab::Directories => self.dir_browser_pane.scroll_offset,
            tab => self.list_panes[list_slot(tab)].scroll_offset,
        }
    }

    pub fn refresh_dir_browser(&mut self, app: &App) {
        self.dir_browser_pane.refresh(app);
    }

    /// Re-run the open search against the new library. `search_modal_results`
    /// holds raw library indices and lives on `Ui`, so `App::replace_library`
    /// never sees it — stale entries would panic the next render.
    pub fn refresh_search_results(&mut self, app: &App) {
        if !self.show_search_modal {
            self.search_modal_results.clear();
            return;
        }
        self.search_modal_results = app.library.search(&self.search_modal_input);
        self.search_modal_selected = 0;
        self.search_modal_scroll = 0;
        self.search_modal_hover_row = None;
    }

    pub fn clamp_selections(&mut self, app: &App) {
        for tab in [Tab::Artists, Tab::Albums, Tab::Genre, Tab::Format, Tab::Playlists] {
            let len = list_pane::row_count(app, tab);
            let pane = &mut self.list_panes[list_slot(tab)];
            pane.selected = pane.selected.min(len.saturating_sub(1));
            pane.scroll_offset = pane.scroll_offset.min(len.saturating_sub(1));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ui() -> Ui {
        Ui::new(
            std::path::PathBuf::from("."),
            ratatui_image::picker::Picker::from_fontsize((8, 16)),
        )
    }

    fn elapsed(ui: &Ui) -> f32 {
        ui.splash_start.map(|s| s.elapsed().as_secs_f32()).unwrap_or(0.0)
    }

    #[test]
    fn a_keypress_skips_the_splash_hold() {
        let mut ui = ui();
        assert!(ui.show_splash);
        assert!(elapsed(&ui) < SPLASH_FADE_OUT_AT);

        ui.begin_splash_fade_out();

        // Straight to the fade-out rather than sitting through the hold.
        assert!(elapsed(&ui) >= SPLASH_FADE_OUT_AT);
        assert!(elapsed(&ui) < SPLASH_END, "the fade-out still plays");
    }

    #[test]
    fn skipping_twice_does_not_cut_the_fade_out_short() {
        let mut ui = ui();
        ui.begin_splash_fade_out();
        let first = elapsed(&ui);
        ui.begin_splash_fade_out();
        assert!(elapsed(&ui) >= first, "must not jump past the end");
        assert!(elapsed(&ui) < SPLASH_END);
    }

    #[test]
    fn skipping_a_dismissed_splash_is_a_no_op() {
        let mut ui = ui();
        ui.show_splash = false;
        ui.splash_start = None;
        ui.begin_splash_fade_out();
        assert!(ui.splash_start.is_none());
    }
}
