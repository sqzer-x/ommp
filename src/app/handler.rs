use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};

use crate::app::state::{FocusedPane, Tab};
use crate::app::{App, AppAction};
use crate::ui::layout::LayoutAreas;
use crate::ui::pane::Pane;
use crate::ui::widgets::{progress_bar, tab_bar};
use crate::ui::widgets::playlist_modal::PlaylistModalMode;
use crate::ui::Ui;

pub const MIN_PANE_WIDTH: u16 = 10;

/// Split the 100% budget left over after `fixed` between the two panes flanking
/// a dragged border, placing the boundary at `boundary_pct`. Returns None when
/// there is not enough room for both.
///
/// Every arm of the drag math used to do this inline with bare `u16`
/// subtraction, which underflows as soon as the pointer leaves the dashboard —
/// dragging the playlist|lyrics border to the left edge panicked outright.
fn split_widths(boundary_pct: u16, fixed: u16, min_w: u16) -> Option<(u16, u16)> {
    let budget = 100u16.checked_sub(fixed)?;
    if budget < min_w * 2 {
        return None;
    }
    let first = boundary_pct.clamp(min_w, budget - min_w);
    Some((first, budget - first))
}

pub fn handle_key_event(key: KeyEvent, app: &App, ui: &mut Ui) -> Vec<AppAction> {
    let mut actions = Vec::new();

    // About modal: Esc to close, g/s to open URLs
    if ui.show_about_modal {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                ui.show_about_modal = false;
            }
            KeyCode::Char('g') => {
                let _ = std::process::Command::new("xdg-open")
                    .arg("https://github.com/sqzer-x/ommp")
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
            }
            KeyCode::Char('s') => {
                let _ = std::process::Command::new("xdg-open")
                    .arg("https://github.com/sponsors/sqzer-x")
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
            }
            _ => {}
        }
        return actions;
    }

    // Help modal: Esc to close
    if ui.show_help_modal {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
            ui.show_help_modal = false;
        }
        return actions;
    }

    // Playlist modal ("b" key) — list, create, rename modes
    if ui.show_playlist_modal {
        match ui.playlist_modal_mode {
            PlaylistModalMode::Create | PlaylistModalMode::Rename => {
                match key.code {
                    KeyCode::Esc => {
                        ui.playlist_modal_mode = PlaylistModalMode::List;
                        ui.playlist_modal_input.clear();
                    }
                    KeyCode::Enter => {
                        let name = ui.playlist_modal_input.trim().to_string();
                        if !name.is_empty() {
                            if ui.playlist_modal_mode == PlaylistModalMode::Create {
                                actions.push(AppAction::CreatePlaylist(name));
                            } else {
                                actions.push(AppAction::RenamePlaylist {
                                    idx: ui.playlist_modal_selected,
                                    name,
                                });
                            }
                        }
                        ui.playlist_modal_mode = PlaylistModalMode::List;
                        ui.playlist_modal_input.clear();
                    }
                    KeyCode::Backspace => {
                        ui.playlist_modal_input.pop();
                    }
                    KeyCode::Char(c) => {
                        ui.playlist_modal_input.push(c);
                    }
                    _ => {}
                }
            }
            PlaylistModalMode::List => {
                match key.code {
                    KeyCode::Esc => {
                        ui.show_playlist_modal = false;
                        ui.playlist_modal_selected = 0;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if ui.playlist_modal_selected > 0 {
                            ui.playlist_modal_selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !app.playlists.is_empty()
                            && ui.playlist_modal_selected < app.playlists.len() - 1
                        {
                            ui.playlist_modal_selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        // Toggle track in selected playlist
                        if let Some(track_idx) = app.queue.current_index
                            .and_then(|qi| app.queue.tracks.get(qi).copied())
                        {
                            let pl_idx = ui.playlist_modal_selected;
                            if pl_idx < app.playlists.len() {
                                if app.playlists[pl_idx].tracks.contains(&track_idx) {
                                    actions.push(AppAction::RemoveFromPlaylist {
                                        playlist_idx: pl_idx,
                                        track_idx,
                                    });
                                } else {
                                    actions.push(AppAction::AddToPlaylist {
                                        playlist_idx: pl_idx,
                                        track_idx,
                                    });
                                }
                            }
                        }
                    }
                    KeyCode::Char('a') => {
                        ui.playlist_modal_mode = PlaylistModalMode::Create;
                        ui.playlist_modal_input.clear();
                    }
                    KeyCode::Char('d') => {
                        if !app.playlists.is_empty() {
                            actions.push(AppAction::DeletePlaylist(ui.playlist_modal_selected));
                            if ui.playlist_modal_selected > 0
                                && ui.playlist_modal_selected >= app.playlists.len() - 1
                            {
                                ui.playlist_modal_selected -= 1;
                            }
                        }
                    }
                    KeyCode::Char('r')
                        if !app.playlists.is_empty() => {
                            ui.playlist_modal_mode = PlaylistModalMode::Rename;
                            ui.playlist_modal_input =
                                app.playlists[ui.playlist_modal_selected].name.clone();
                        }
                    _ => {}
                }
            }
        }
        return actions;
    }

    // Search modal: input handling
    if ui.show_search_modal {
        match key.code {
            KeyCode::Esc => {
                ui.show_search_modal = false;
                ui.search_modal_input.clear();
                ui.search_modal_results.clear();
                ui.search_modal_selected = 0;
                ui.search_modal_scroll = 0;
            }
            KeyCode::Enter => {
                if !ui.search_modal_results.is_empty() {
                    let track_idx = ui.search_modal_results[ui.search_modal_selected];
                    actions.push(AppAction::AddToQueue(vec![track_idx]));
                    ui.show_search_modal = false;
                    ui.search_modal_input.clear();
                    ui.search_modal_results.clear();
                    ui.search_modal_selected = 0;
                    ui.search_modal_scroll = 0;
                }
            }
            KeyCode::Up | KeyCode::BackTab => {
                if ui.search_modal_selected > 0 {
                    ui.search_modal_selected -= 1;
                    if ui.search_modal_selected < ui.search_modal_scroll {
                        ui.search_modal_scroll = ui.search_modal_selected;
                    }
                }
            }
            KeyCode::Down | KeyCode::Tab => {
                if !ui.search_modal_results.is_empty()
                    && ui.search_modal_selected < ui.search_modal_results.len() - 1
                {
                    ui.search_modal_selected += 1;
                    let h = ui.search_modal_result_height;
                    if h > 0 && ui.search_modal_selected >= ui.search_modal_scroll + h {
                        ui.search_modal_scroll = ui.search_modal_selected - h + 1;
                    }
                }
            }
            KeyCode::Backspace => {
                ui.search_modal_input.pop();
                ui.search_modal_results = app.library.search(&ui.search_modal_input);
                ui.search_modal_selected = 0;
                ui.search_modal_scroll = 0;
            }
            KeyCode::Char(c) => {
                ui.search_modal_input.push(c);
                ui.search_modal_results = app.library.search(&ui.search_modal_input);
                ui.search_modal_selected = 0;
                ui.search_modal_scroll = 0;
            }
            _ => {}
        }
        return actions;
    }

    // In search input mode, ignore (search handled by modal now)
    if app.search_mode {
        return actions;
    }

    // Chord: Ctrl+E pressed, waiting for next key
    if ui.chord_pending {
        ui.chord_pending = false;
        match key.code {
            KeyCode::Char('s') => {
                ui.show_search_modal = true;
            }
            KeyCode::Char('h') => {
                ui.show_help_modal = true;
            }
            KeyCode::Char('r') => {
                ui.resize_mode = !ui.resize_mode;
            }
            KeyCode::Char('i') => {
                ui.show_about_modal = true;
            }
            KeyCode::Char('l') => {
                actions.push(AppAction::LibrarySync);
            }
            _ => {} // unknown chord, ignore
        }
        return actions;
    }

    // Ctrl+E → chord pending
    if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('e') {
        ui.chord_pending = true;
        return actions;
    }

    // Resize mode key handling
    if ui.resize_mode {
        match key.code {
            KeyCode::Char('h') | KeyCode::Left => {
                resize_pane(ui, app.focus, -2);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                resize_pane(ui, app.focus, 2);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                // Grow info pane (shrink lyrics)
                let new_split = (ui.right_split as i16 - 3).clamp(10, 90) as u16;
                ui.right_split = new_split;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                // Grow lyrics (shrink info pane)
                let new_split = (ui.right_split as i16 + 3).clamp(10, 90) as u16;
                ui.right_split = new_split;
            }
            KeyCode::Esc | KeyCode::Enter => {
                ui.resize_mode = false;
            }
            KeyCode::Char('q') => {
                ui.resize_mode = false;
                actions.push(AppAction::Quit);
            }
            _ => {}
        }
        return actions;
    }

    // Global keybindings first
    match (key.modifiers, key.code) {
        (_, KeyCode::Char('q')) => {
            actions.push(AppAction::Quit);
            return actions;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            actions.push(AppAction::Quit);
            return actions;
        }
        (_, KeyCode::Char(' ')) => {
            actions.push(AppAction::PauseResume);
            return actions;
        }
        (_, KeyCode::Char('n')) => {
            actions.push(AppAction::NextTrack);
            return actions;
        }
        (KeyModifiers::SHIFT, KeyCode::Char('N')) => {
            actions.push(AppAction::PrevTrack);
            return actions;
        }
        (_, KeyCode::Char('+')) | (_, KeyCode::Char('=')) => {
            actions.push(AppAction::VolumeUp);
            return actions;
        }
        (_, KeyCode::Char('-')) => {
            actions.push(AppAction::VolumeDown);
            return actions;
        }
        (_, KeyCode::Right) => {
            actions.push(AppAction::SeekForward);
            return actions;
        }
        (_, KeyCode::Left) => {
            actions.push(AppAction::SeekBackward);
            return actions;
        }
        (_, KeyCode::Char('s')) => {
            actions.push(AppAction::ToggleShuffle);
            return actions;
        }
        (_, KeyCode::Char('r')) => {
            actions.push(AppAction::CycleRepeat);
            return actions;
        }
        (_, KeyCode::Char('b')) => {
            // Only open if a track is playing
            if app.queue.current_index.is_some() {
                ui.show_playlist_modal = true;
                ui.playlist_modal_selected = 0;
            }
            return actions;
        }
        (_, KeyCode::Char('p')) => {
            ui.info_view = ui.info_view.next();
            return actions;
        }
        (_, KeyCode::Tab) => {
            actions.push(AppAction::FocusNext);
            return actions;
        }
        (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            actions.push(AppAction::FocusPrev);
            return actions;
        }
        // Tab switching with number keys
        (_, KeyCode::Char('1')) => {
            actions.push(AppAction::SwitchTab(Tab::Queue));
            return actions;
        }
        (_, KeyCode::Char('2')) => {
            actions.push(AppAction::SwitchTab(Tab::Directories));
            return actions;
        }
        (_, KeyCode::Char('3')) => {
            actions.push(AppAction::SwitchTab(Tab::Artists));
            return actions;
        }
        (_, KeyCode::Char('4')) => {
            actions.push(AppAction::SwitchTab(Tab::Albums));
            return actions;
        }
        (_, KeyCode::Char('5')) => {
            actions.push(AppAction::SwitchTab(Tab::Genre));
            return actions;
        }
        (_, KeyCode::Char('6')) => {
            actions.push(AppAction::SwitchTab(Tab::Format));
            return actions;
        }
        (_, KeyCode::Char('7')) => {
            actions.push(AppAction::SwitchTab(Tab::Playlists));
            return actions;
        }
        // h/l for pane focus
        (_, KeyCode::Char('h')) => {
            actions.push(AppAction::FocusPrev);
            return actions;
        }
        (_, KeyCode::Char('l')) => {
            actions.push(AppAction::FocusNext);
            return actions;
        }
        _ => {}
    }

    // Route to focused pane
    let action = match app.focus {
        FocusedPane::Library => match app.tab {
            Tab::Queue => ui.library_pane.handle_key(key, app),
            Tab::Directories => ui.dir_browser_pane.handle_key(key, app),
            Tab::Artists => ui.artists_pane.handle_key(key, app),
            Tab::Albums => ui.albums_pane.handle_key(key, app),
            Tab::Genre => ui.genre_pane.handle_key(key, app),
            Tab::Format => ui.format_pane.handle_key(key, app),
            Tab::Playlists => ui.playlists_pane.handle_key(key, app),
        },
        FocusedPane::Playlist => {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => None,
                KeyCode::Char('k') | KeyCode::Up => None,
                _ => ui.queue_pane.handle_key(key, app),
            }
        }
        FocusedPane::Lyrics => ui.lyrics_pane.handle_key(key, app),
    };

    // Auto-focus to Queue pane when adding tracks from Library
    if let Some(ref a) = action {
        if matches!(a, AppAction::AddToQueue(_)) && app.focus == FocusedPane::Library {
            actions.push(AppAction::FocusPane(FocusedPane::Playlist));
        }
    }

    if let Some(a) = action {
        actions.push(a);
    }

    actions
}

pub fn handle_mouse_event(
    mouse: MouseEvent,
    app: &App,
    ui: &mut Ui,
    terminal_area: ratatui::layout::Rect,
) -> Vec<AppAction> {
    let mut actions = Vec::new();
    let areas = LayoutAreas::compute(terminal_area, ui.pane_widths, ui.right_split);

    let x = mouse.column;
    let y = mouse.row;

    // Store mouse position for hover tracking across all event types
    ui.mouse_pos = Some((x, y));

    // Search modal mouse handling
    if ui.show_search_modal {
        let ra = ui.search_modal_result_area;
        let in_results = x >= ra.x && x < ra.x + ra.width
            && y >= ra.y && y < ra.y + ra.height;

        // Hover tracking
        if in_results && !ui.search_modal_results.is_empty() {
            let row = ui.search_modal_scroll + (y - ra.y) as usize;
            if row < ui.search_modal_results.len() {
                ui.search_modal_hover_row = Some(row);
            } else {
                ui.search_modal_hover_row = None;
            }
        } else {
            ui.search_modal_hover_row = None;
        }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if in_results && !ui.search_modal_results.is_empty() {
                    let clicked = ui.search_modal_scroll + (y - ra.y) as usize;
                    if clicked < ui.search_modal_results.len() {
                        let is_double = is_double_click(ui.last_click, x, y);
                        ui.last_click = if is_double {
                            None
                        } else {
                            Some((Instant::now(), x, y))
                        };

                        if is_double {
                            // Double-click: select and confirm (add to queue)
                            let track_idx = ui.search_modal_results[clicked];
                            actions.push(AppAction::AddToQueue(vec![track_idx]));
                            ui.show_search_modal = false;
                            ui.search_modal_input.clear();
                            ui.search_modal_results.clear();
                            ui.search_modal_selected = 0;
                            ui.search_modal_scroll = 0;
                        } else {
                            // Single click: select
                            ui.search_modal_selected = clicked;
                        }
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if in_results && !ui.search_modal_results.is_empty() {
                    let max_scroll = ui.search_modal_results.len()
                        .saturating_sub(ui.search_modal_result_height);
                    if ui.search_modal_scroll < max_scroll {
                        ui.search_modal_scroll += 1;
                    }
                }
            }
            MouseEventKind::ScrollUp
                if in_results && ui.search_modal_scroll > 0 => {
                    ui.search_modal_scroll -= 1;
                }
            _ => {}
        }
        return actions;
    }

    // Block all mouse events when any other modal is open
    if ui.show_about_modal || ui.show_help_modal || ui.show_playlist_modal {
        return actions;
    }

    // Determine which pane the mouse is in
    let hit = PaneHit::at(&areas, x, y);
    let in_library = hit.library;
    let in_playlist = hit.playlist;
    let in_lyrics = hit.lyrics;

    // --- Hover tracking (runs on every mouse event including Moved) ---
    update_hover(ui, &areas, app, x, y, in_library, in_playlist);
    update_tab_hover(ui, &areas, x, y);

    // Focus follows the click, not the pointer. Deriving it from hover meant a
    // scroll — or a tick replaying the last known position — immediately undid
    // Tab, and undid the auto-focus after adding to the queue.
    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
        if let Some(pane) = hit.pane() {
            if app.focus != pane {
                actions.push(AppAction::FocusPane(pane));
            }
        }
    }

    // --- Border drag resize ---
    let border0_x = areas.library.x + areas.library.width; // lib|playlist boundary
    let border1_x = areas.playlist.x + areas.playlist.width; // playlist|lyrics boundary
    let in_dashboard_y = y >= areas.library.y && y < areas.library.y + areas.library.height;

    // Handle active drag (before normal mouse processing)
    if ui.dragging_border.is_some() {
        match mouse.kind {
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved => {
                if let Some(border_idx) = ui.dragging_border {
                    let dashboard_x = areas.library.x;
                    let dashboard_w = areas.library.width + areas.playlist.width + areas.lyrics.width;
                    if dashboard_w > 0 {
                        let rel_x = x.saturating_sub(dashboard_x);
                        let pct = ((rel_x as u32 * 100) / dashboard_w as u32) as u16;
                        let min_w: u16 = MIN_PANE_WIDTH;
                        if border_idx == 0 {
                            // Dragging lib|playlist border
                            if let Some((lib, play)) = split_widths(pct, ui.pane_widths[2], min_w) {
                                ui.pane_widths[0] = lib;
                                ui.pane_widths[1] = play;
                            }
                        } else if border_idx == 1 {
                            // Dragging playlist|lyrics border. `pct` is measured
                            // from the dashboard's left edge, so subtract the
                            // library pane to get the boundary inside the budget.
                            let boundary = pct.saturating_sub(ui.pane_widths[0]);
                            if let Some((play, right)) =
                                split_widths(boundary, ui.pane_widths[0], min_w)
                            {
                                ui.pane_widths[1] = play;
                                ui.pane_widths[2] = right;
                            }
                        } else if border_idx == 2 {
                            // Dragging info|lyrics horizontal border
                            let right_top = areas.info_pane.y;
                            let right_h = areas.info_pane.height + areas.lyrics.height;
                            if right_h > 0 {
                                let rel_y = y.saturating_sub(right_top);
                                let pct_v = ((rel_y as u32 * 100) / right_h as u32).clamp(10, 90) as u16;
                                ui.right_split = pct_v;
                            }
                        }
                    }
                }
                return actions;
            }
            MouseEventKind::Up(MouseButton::Left) => {
                ui.dragging_border = None;
                return actions;
            }
            _ => {
                ui.dragging_border = None;
            }
        }
    }

    // --- Handle specific event kinds ---
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Border drag start detection
            if in_dashboard_y {
                if x.abs_diff(border0_x) <= 1 {
                    ui.dragging_border = Some(0);
                    return actions;
                }
                if x.abs_diff(border1_x) <= 1 {
                    ui.dragging_border = Some(1);
                    return actions;
                }
                // Horizontal border between info_pane and lyrics (within right column)
                let border2_y = areas.info_pane.y + areas.info_pane.height;
                if x >= areas.info_pane.x
                    && x < areas.info_pane.x + areas.info_pane.width
                    && y.abs_diff(border2_y) <= 1
                {
                    ui.dragging_border = Some(2);
                    return actions;
                }
            }

            // Double-click detection. The column has to match too: comparing
            // only the row made a click in the library followed by a click on
            // the same screen row of the queue register as a double-click.
            let is_double_click = is_double_click(ui.last_click, x, y);
            // Cleared once it fires, so three fast clicks are one double-click
            // and one single, not two doubles.
            ui.last_click = if is_double_click {
                None
            } else {
                Some((Instant::now(), x, y))
            };

            // Tab bar click
            if y >= areas.tab_bar.y && y < areas.tab_bar.y + areas.tab_bar.height {
                if let Some(tab_idx) = tab_bar::tab_hit_test(areas.tab_bar, x) {
                    actions.push(AppAction::SwitchTab(Tab::from_index(tab_idx)));
                }
                return actions;
            }

            // Progress bar click
            if y >= areas.progress_bar.y && y < areas.progress_bar.y + areas.progress_bar.height {
                let gauge_area = progress_bar::progress_gauge_area(areas.progress_bar);
                if x >= gauge_area.x && x < gauge_area.x + gauge_area.width {
                    let ratio = (x - gauge_area.x) as f64 / gauge_area.width as f64;
                    let seek_pos = ratio * app.playback.duration_secs;
                    actions.push(AppAction::Seek(seek_pos));
                }
                return actions;
            }

            // Double-click in playlist → play that track
            if is_double_click && in_playlist {
                let block = ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL);
                let inner = block.inner(areas.playlist);
                if y >= inner.y && y < inner.y + inner.height {
                    let clicked = ui.queue_pane.scroll_offset + (y - inner.y) as usize;
                    if clicked < app.queue.tracks.len() {
                        actions.push(AppAction::PlayQueueIndex(clicked));
                        return actions;
                    }
                }
            }

            // Single click in library → select + activate (Enter).
            // Only when the click actually landed on a row: the synthesized
            // Enter is unconditional, so a click on the border or the blank
            // space under a short list used to activate whatever was selected
            // before — which with a queue-replacing activation meant one stray
            // click wiped the queue.
            let clicked_library_row = clicked_row_in(areas.library, y)
                .map(|row| ui.library_pane_scroll(app) + row)
                .filter(|&row| row < library_row_count(app, ui));
            if in_library && clicked_library_row.is_some() {
                // First, route mouse to pane for selection update
                let _sel_action = match app.tab {
                    Tab::Queue => ui.library_pane.handle_mouse(mouse, areas.library, app),
                    Tab::Directories => ui.dir_browser_pane.handle_mouse(mouse, areas.library, app),
                    Tab::Artists => ui.artists_pane.handle_mouse(mouse, areas.library, app),
                    Tab::Albums => ui.albums_pane.handle_mouse(mouse, areas.library, app),
                    Tab::Genre => ui.genre_pane.handle_mouse(mouse, areas.library, app),
                    Tab::Format => ui.format_pane.handle_mouse(mouse, areas.library, app),
                    Tab::Playlists => ui.playlists_pane.handle_mouse(mouse, areas.library, app),
                };
                // Then, trigger Enter action to activate the clicked item
                let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
                let activate_action = match app.tab {
                    Tab::Queue => ui.library_pane.handle_key(enter_key, app),
                    Tab::Directories => ui.dir_browser_pane.handle_key(enter_key, app),
                    Tab::Artists => ui.artists_pane.handle_key(enter_key, app),
                    Tab::Albums => ui.albums_pane.handle_key(enter_key, app),
                    Tab::Genre => ui.genre_pane.handle_key(enter_key, app),
                    Tab::Format => ui.format_pane.handle_key(enter_key, app),
                    Tab::Playlists => ui.playlists_pane.handle_key(enter_key, app),
                };
                if let Some(action) = activate_action {
                    if matches!(action, AppAction::AddToQueue(_)) {
                        actions.push(AppAction::FocusPane(FocusedPane::Playlist));
                    }
                    actions.push(action);
                }
            } else if in_playlist {
                if let Some(a) = ui.queue_pane.handle_mouse(mouse, areas.playlist, app) {
                    actions.push(a);
                }
                // Update queue selection on click
                let block = ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL);
                let inner = block.inner(areas.playlist);
                if y >= inner.y && y < inner.y + inner.height {
                    let clicked = ui.queue_pane.scroll_offset + (y - inner.y) as usize;
                    if clicked < app.queue.tracks.len() {
                        actions.push(AppAction::SetQueueSelection(clicked));
                    }
                }
            } else if in_lyrics {
                if let Some(a) = ui.lyrics_pane.handle_mouse(mouse, areas.lyrics, app) {
                    actions.push(a);
                }
            }
        }
        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
            if in_library {
                let action = match app.tab {
                    Tab::Queue => ui.library_pane.handle_mouse(mouse, areas.library, app),
                    Tab::Directories => ui.dir_browser_pane.handle_mouse(mouse, areas.library, app),
                    Tab::Artists => ui.artists_pane.handle_mouse(mouse, areas.library, app),
                    Tab::Albums => ui.albums_pane.handle_mouse(mouse, areas.library, app),
                    Tab::Genre => ui.genre_pane.handle_mouse(mouse, areas.library, app),
                    Tab::Format => ui.format_pane.handle_mouse(mouse, areas.library, app),
                    Tab::Playlists => ui.playlists_pane.handle_mouse(mouse, areas.library, app),
                };
                if let Some(a) = action {
                    actions.push(a);
                }
            } else if in_playlist {
                if let Some(a) = ui.queue_pane.handle_mouse(mouse, areas.playlist, app) {
                    actions.push(a);
                }
            } else if in_lyrics {
                if let Some(a) = ui.lyrics_pane.handle_mouse(mouse, areas.lyrics, app) {
                    actions.push(a);
                }
            }
        }
        _ => {
            // Moved and other events — hover already handled above
        }
    }

    actions
}

const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// Row index under `y` inside a bordered pane, or None for the border itself.
fn clicked_row_in(area: ratatui::layout::Rect, y: u16) -> Option<usize> {
    let inner = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .inner(area);
    (y >= inner.y && y < inner.y + inner.height).then(|| (y - inner.y) as usize)
}

/// How many rows the library pane is currently showing, so a click below the
/// last one can be ignored instead of activating whatever was selected before.
fn library_row_count(app: &App, ui: &Ui) -> usize {
    match app.tab {
        Tab::Queue => ui.library_pane.row_count(app),
        Tab::Directories => ui.dir_browser_pane.entries.len(),
        Tab::Artists => app.library.get_artists().len(),
        Tab::Albums => app.library.get_albums().len(),
        Tab::Genre => app.library.get_genres().len(),
        Tab::Format => app.library.get_formats().len(),
        Tab::Playlists => app.playlists.len(),
    }
}

/// A second click counts only if it lands on the same cell in time. Matching on
/// the row alone let a click in one pane pair up with a click in another.
fn is_double_click(last: Option<(Instant, u16, u16)>, x: u16, y: u16) -> bool {
    last.is_some_and(|(at, lx, ly)| lx == x && ly == y && at.elapsed() < DOUBLE_CLICK)
}

/// Which dashboard pane a point lands in. Both the mouse handler and the tick
/// refresh need this; they used to carry byte-identical copies of the maths.
struct PaneHit {
    library: bool,
    playlist: bool,
    lyrics: bool,
}

impl PaneHit {
    fn at(areas: &LayoutAreas, x: u16, y: u16) -> Self {
        let inside = |r: ratatui::layout::Rect| {
            x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
        };
        Self {
            library: inside(areas.library),
            playlist: inside(areas.playlist),
            lyrics: inside(areas.lyrics),
        }
    }

    fn pane(&self) -> Option<FocusedPane> {
        if self.library {
            Some(FocusedPane::Library)
        } else if self.playlist {
            Some(FocusedPane::Playlist)
        } else if self.lyrics {
            Some(FocusedPane::Lyrics)
        } else {
            None
        }
    }
}

fn update_tab_hover(ui: &mut Ui, areas: &LayoutAreas, x: u16, y: u16) {
    ui.hovered_tab = if y >= areas.tab_bar.y && y < areas.tab_bar.y + areas.tab_bar.height {
        tab_bar::tab_hit_test(areas.tab_bar, x)
    } else {
        None
    };
}

/// Clear all hover_row state across all panes
fn clear_all_hovers(ui: &mut Ui) {
    ui.queue_pane.hover_row = None;
    ui.library_pane.hover_row = None;
    ui.dir_browser_pane.hover_row = None;
    ui.artists_pane.hover_row = None;
    ui.albums_pane.hover_row = None;
    ui.genre_pane.hover_row = None;
    ui.format_pane.hover_row = None;
    ui.playlists_pane.hover_row = None;
}

/// Update hover_row state for panes based on mouse position
fn update_hover(
    ui: &mut Ui,
    areas: &LayoutAreas,
    app: &App,
    x: u16,
    y: u16,
    in_library: bool,
    in_playlist: bool,
) {
    clear_all_hovers(ui);

    if in_playlist {
        let block = ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL);
        let inner = block.inner(areas.playlist);
        if x >= inner.x && x < inner.x + inner.width
            && y >= inner.y && y < inner.y + inner.height
        {
            let row = ui.queue_pane.scroll_offset + (y - inner.y) as usize;
            if row < app.queue.tracks.len() {
                ui.queue_pane.hover_row = Some(row);
            }
        }
    } else if in_library {
        let block = ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL);
        let inner = block.inner(areas.library);
        // Bounded like the queue branch above: hovering the blank space below a
        // short list used to light up a row that isn't there.
        let row_count = library_row_count(app, ui);
        let scroll = ui.library_pane_scroll(app);
        if x >= inner.x && x < inner.x + inner.width
            && y >= inner.y && y < inner.y + inner.height
            && (scroll + (y - inner.y) as usize) < row_count
        {
            let visual_row = (y - inner.y) as usize;
            match app.tab {
                Tab::Queue => {
                    let row = ui.library_pane.scroll_offset + visual_row;
                    ui.library_pane.hover_row = Some(row);
                }
                Tab::Directories => {
                    let row = ui.dir_browser_pane.scroll_offset + visual_row;
                    ui.dir_browser_pane.hover_row = Some(row);
                }
                Tab::Artists => {
                    let row = ui.artists_pane.scroll_offset + visual_row;
                    ui.artists_pane.hover_row = Some(row);
                }
                Tab::Albums => {
                    let row = ui.albums_pane.scroll_offset + visual_row;
                    ui.albums_pane.hover_row = Some(row);
                }
                Tab::Genre => {
                    let row = ui.genre_pane.scroll_offset + visual_row;
                    ui.genre_pane.hover_row = Some(row);
                }
                Tab::Format => {
                    let row = ui.format_pane.scroll_offset + visual_row;
                    ui.format_pane.hover_row = Some(row);
                }
                Tab::Playlists => {
                    let row = ui.playlists_pane.scroll_offset + visual_row;
                    ui.playlists_pane.hover_row = Some(row);
                }
            }
        }
    }
}

/// Refresh row highlighting from the stored mouse position, so hover stays
/// current on terminals that only report motion sporadically.
///
/// This deliberately does not touch focus. It used to re-derive focus from
/// `ui.mouse_pos` on every 200ms tick, and since that position is latched on the
/// first mouse event and never cleared, pressing Tab moved focus for one frame
/// before the next tick dragged it back under the pointer.
pub fn refresh_hover(app: &App, ui: &mut Ui, terminal_area: ratatui::layout::Rect) {
    // Skip hover updates when any modal is open
    if ui.show_about_modal || ui.show_help_modal || ui.show_search_modal || ui.show_playlist_modal {
        return;
    }
    let Some((x, y)) = ui.mouse_pos else { return };
    let areas = LayoutAreas::compute(terminal_area, ui.pane_widths, ui.right_split);
    let hit = PaneHit::at(&areas, x, y);
    update_hover(ui, &areas, app, x, y, hit.library, hit.playlist);
    update_tab_hover(ui, &areas, x, y);
}

/// Resize the focused pane by delta percentage points.
/// Positive delta = grow the focused pane rightward, negative = shrink rightward.
fn resize_pane(ui: &mut Ui, focus: FocusedPane, delta: i16) {
    let min_width: u16 = 10;
    let w = &mut ui.pane_widths;

    match focus {
        FocusedPane::Library => {
            let new_lib = (w[0] as i16 + delta).clamp(min_width as i16, 80) as u16;
            let diff = new_lib as i16 - w[0] as i16;
            let new_play = (w[1] as i16 - diff).max(min_width as i16) as u16;
            let actual_diff = w[1] as i16 - new_play as i16;
            w[0] = (w[0] as i16 + actual_diff) as u16;
            w[1] = new_play;
        }
        FocusedPane::Playlist => {
            if delta < 0 {
                let shrink = (-delta) as u16;
                if w[1] > min_width + shrink - 1 {
                    w[1] -= shrink;
                    w[0] += shrink;
                }
            } else {
                let grow = delta as u16;
                if w[2] > min_width + grow - 1 {
                    w[2] -= grow;
                    w[1] += grow;
                }
            }
        }
        FocusedPane::Lyrics => {
            let new_lyr = (w[2] as i16 - delta).clamp(min_width as i16, 80) as u16;
            let diff = w[2] as i16 - new_lyr as i16;
            let new_play = (w[1] as i16 + diff).max(min_width as i16) as u16;
            let actual_diff = new_play as i16 - w[1] as i16;
            w[2] = (w[2] as i16 - actual_diff) as u16;
            w[1] = new_play;
        }
    }
}

/// Update queue selection based on keyboard in playlist focus
pub fn update_queue_selection(app: &mut App, key: KeyEvent) {
    let count = app.queue.tracks.len();
    if count == 0 {
        return;
    }

    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.queue.selected_index < count - 1 {
                app.queue.selected_index += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up
            if app.queue.selected_index > 0 => {
                app.queue.selected_index -= 1;
            }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_widths_keeps_the_total_at_100_and_both_panes_above_the_minimum() {
        let (a, b) = split_widths(50, 20, MIN_PANE_WIDTH).unwrap();
        assert_eq!((a, b), (50, 30));
        assert_eq!(a + b + 20, 100);
    }

    #[test]
    fn split_widths_survives_dragging_past_either_edge() {
        // Pointer at or left of the dashboard: used to underflow `100 - fixed - 100`.
        let (a, b) = split_widths(0, 20, MIN_PANE_WIDTH).unwrap();
        assert_eq!((a, b), (MIN_PANE_WIDTH, 80 - MIN_PANE_WIDTH));
        // Pointer dragged far past the right edge.
        let (a, b) = split_widths(500, 20, MIN_PANE_WIDTH).unwrap();
        assert_eq!((a, b), (80 - MIN_PANE_WIDTH, MIN_PANE_WIDTH));
    }

    #[test]
    fn split_widths_refuses_when_the_third_pane_leaves_no_room() {
        // 85 fixed leaves 15, which cannot hold two 10% panes. The old code
        // called clamp(10, 5) here, and `Ord::clamp` panics when min > max.
        assert_eq!(split_widths(50, 85, MIN_PANE_WIDTH), None);
        assert_eq!(split_widths(50, 120, MIN_PANE_WIDTH), None);
        // Exactly enough room is allowed.
        assert_eq!(split_widths(50, 80, MIN_PANE_WIDTH), Some((10, 10)));
    }
}
