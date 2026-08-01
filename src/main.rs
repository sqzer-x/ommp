mod app;
mod audio;
mod event;
mod library;
mod ui;

use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::handler;
use app::persist;
use app::state::{FocusedPane, InfoView, RepeatMode};
use app::App;
use audio::AudioEngine;
use event::input;
use event::{AudioEvent, Event};

fn setup_terminal() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    // Explicitly enable mouse motion tracking (SGR any-event mode)
    // Some terminals need this even after EnableMouseCapture
    stdout.write_all(b"\x1b[?1003h")?;
    stdout.flush()?;
    Ok(())
}

/// Undo everything `setup_terminal` did. Best-effort and idempotent so the
/// panic hook can call it without caring how far setup got.
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        // Disable mouse motion tracking
        crossterm::style::Print("\x1b[?1003l"),
        LeaveAlternateScreen,
        DisableMouseCapture,
        crossterm::cursor::Show,
    );
}

fn main() -> Result<()> {
    setup_terminal()?;

    // Restore before the default hook prints: a panic message written while the
    // alternate screen is up scrolls away with it, leaving a raw-mode terminal
    // and no explanation.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal);

    restore_terminal();

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let music_dir = dirs_music_path();

    // Detect terminal image protocol BEFORE input thread steals stdin
    let picker = ratatui_image::picker::Picker::from_query_stdio()
        .unwrap_or_else(|_| ratatui_image::picker::Picker::from_fontsize((8, 16)));

    // Event channel
    let (event_tx, event_rx) = crossbeam_channel::unbounded();

    // Spawn input thread
    let _input_handle = input::spawn_input_thread(event_tx.clone());
    let _tick_handle = input::spawn_tick_thread(event_tx.clone(), Duration::from_millis(200));

    // Audio engine
    let audio_engine = AudioEngine::new(event_tx.clone())?;

    // App state
    let mut app = App::new(music_dir.clone());
    app.set_audio_engine(audio_engine);
    app.set_event_tx(event_tx.clone());

    // Scan library in background
    let scan_dir = music_dir.clone();
    let scan_handle = std::thread::spawn(move || {
        library::Library::scan(&scan_dir)
    });

    // UI
    let mut ui = ui::Ui::new(music_dir.clone(), picker);

    // Initial render
    terminal.draw(|frame| {
        ui.render(frame, &app);
    })?;

    // Wait for library scan to complete (non-blocking check in event loop)
    let mut scan_done = false;
    let mut scan_join = Some(scan_handle);
    let mut _watcher: Option<notify::RecommendedWatcher> = None;
    // A backend write error must not skip the save below, so the loop records it
    // and breaks instead of returning early.
    let mut draw_error: Option<io::Error> = None;

    loop {
        // Check if library scan is done
        if !scan_done {
            if let Some(ref handle) = scan_join {
                if handle.is_finished() {
                    if let Some(handle) = scan_join.take() {
                        match handle.join() {
                            Ok(lib) => {
                                app.library = lib;
                                // Load all tracks into queue by default
                                let all_indices: Vec<usize> = (0..app.library.tracks.len()).collect();
                                app.handle_action(app::AppAction::ReplaceQueue(all_indices));
                                ui.refresh_dir_browser(&app);

                                // Restore persisted state
                                if let Some(saved) = persist::load() {
                                    app.playback.volume = saved.volume.clamp(0.0, 1.0);
                                    app.playback.shuffle = saved.shuffle;
                                    app.playback.repeat = RepeatMode::from_label(&saved.repeat);
                                    app.handle_action(app::AppAction::SetVolume(app.playback.volume));
                                    ui.pane_widths = persist::sane_pane_widths(saved.pane_widths);
                                    ui.info_view = InfoView::from_label(&saved.info_view);
                                    ui.right_split = saved.right_split.clamp(10, 90);
                                    // Restore playlists (path → index remapping).
                                    // Paths the library does not have are kept as
                                    // `missing` rather than dropped — launching once
                                    // with the drive unplugged must not delete them.
                                    let mut playlists = Vec::new();
                                    for sp in &saved.playlists {
                                        let mut tracks = Vec::new();
                                        let mut missing = Vec::new();
                                        for p in &sp.tracks {
                                            match app.library.path_to_index(p) {
                                                Some(idx) => tracks.push(idx),
                                                None => missing.push(p.clone()),
                                            }
                                        }
                                        playlists.push(app::state::Playlist {
                                            name: sp.name.clone(),
                                            tracks,
                                            missing,
                                        });
                                    }
                                    if playlists.is_empty() {
                                        playlists.push(app::state::Playlist::new("Bookmarks"));
                                    }
                                    app.playlists = playlists;
                                }

                                scan_done = true;
                                app.initial_scan_complete = true;
                                _watcher = library::watcher::spawn_watcher(&music_dir, event_tx.clone());
                            }
                            Err(_) => {
                                scan_done = true;
                                app.initial_scan_complete = true;
                            }
                        }
                    }
                }
            }
        }

        // Process events
        match event_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => {
                let actions = match event {
                    Event::Key(key) => {
                        // On key press during splash: jump to fade-out phase
                        if ui.show_splash {
                            if let Some(start) = ui.splash_start {
                                let elapsed = start.elapsed().as_secs_f32();
                                if elapsed < 1.5 {
                                    // Jump timeline to start of fade-out (1.5s mark).
                                    // checked_sub because Instant is CLOCK_MONOTONIC:
                                    // started within 1.5s of boot, plain `-` panics.
                                    if let Some(t) = std::time::Instant::now()
                                        .checked_sub(Duration::from_millis(1500))
                                    {
                                        ui.splash_start = Some(t);
                                    }
                                }
                            }
                            vec![]
                        } else {
                        // Handle queue selection directly for playlist focus
                        // Skip when any modal is open
                        if app.focus == FocusedPane::Playlist
                            && !app.search_mode
                            && !ui.show_search_modal
                            && !ui.show_help_modal
                            && !ui.show_playlist_modal
                            && !ui.resize_mode
                            && !ui.chord_pending
                        {
                            handler::update_queue_selection(&mut app, key);
                        }
                        handler::handle_key_event(key, &app, &mut ui)
                        }
                    }
                    Event::Mouse(mouse) => {
                        let size = terminal.size().unwrap_or_default();
                        let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                        handler::handle_mouse_event(mouse, &app, &mut ui, area)
                    }
                    Event::Resize(_, _) => {
                        vec![] // Will re-render on next loop
                    }
                    Event::Tick => {
                        // Auto-dismiss splash after full timeline (2s)
                        if ui.show_splash {
                            if let Some(start) = ui.splash_start {
                                if start.elapsed().as_secs_f32() >= 2.0 {
                                    ui.show_splash = false;
                                    ui.splash_start = None;
                                }
                            }
                        }
                        // Refresh hover from stored mouse position
                        let size = terminal.size().unwrap_or_default();
                        let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                        handler::refresh_hover(&app, &mut ui, area)
                    }
                    Event::LibraryReady(new_lib) => {
                        app.replace_library(new_lib);
                        ui.refresh_dir_browser(&app);
                        ui.clamp_selections(&app);
                        // The search modal caches raw library indices, which
                        // replace_library does not remap; rendering them against
                        // a shrunken library panics.
                        ui.refresh_search_results(&app);
                        vec![]
                    }
                    Event::Audio(audio_event) => {
                        match audio_event {
                            AudioEvent::PositionUpdate {
                                position_secs,
                                duration_secs,
                            } => vec![app::AppAction::UpdatePosition {
                                position_secs,
                                duration_secs,
                            }],
                            AudioEvent::TrackFinished => vec![app::AppAction::TrackFinished],
                            AudioEvent::TrackError(_) => vec![app::AppAction::TrackFailed],
                            AudioEvent::DeviceError(msg) => {
                                app.audio_error = Some(msg);
                                app.playback.state = app::state::PlayState::Stopped;
                                vec![]
                            }
                            AudioEvent::Playing => {
                                app.playback.state = app::state::PlayState::Playing;
                                app.note_playback_started();
                                vec![]
                            }
                            AudioEvent::Paused => {
                                app.playback.state = app::state::PlayState::Paused;
                                vec![]
                            }
                            AudioEvent::Stopped => {
                                app.playback.state = app::state::PlayState::Stopped;
                                vec![]
                            }
                        }
                    }
                };

                for action in actions {
                    app.handle_action(action);
                }

                if app.should_quit {
                    break;
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                // Just re-render
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                break;
            }
        }

        // Render
        if let Err(e) = terminal.draw(|frame| {
            ui.render(frame, &app);
        }) {
            draw_error = Some(e);
            break;
        }
    }

    // Save state on exit
    let saved_playlists: Vec<persist::SavedPlaylist> = app.playlists.iter().map(|pl| {
        persist::SavedPlaylist {
            name: pl.name.clone(),
            // Write the unresolved paths back out too, otherwise a session that
            // ran without the music drive would persist the pruned playlist.
            tracks: pl.tracks.iter()
                .filter_map(|&idx| app.library.tracks.get(idx).map(|t| t.path.clone()))
                .chain(pl.missing.iter().cloned())
                .collect(),
        }
    }).collect();

    let saved = persist::SavedState {
        volume: app.playback.volume,
        shuffle: app.playback.shuffle,
        repeat: app.playback.repeat.as_str().to_string(),
        pane_widths: ui.pane_widths,
        playlists: saved_playlists,
        info_view: ui.info_view.as_str().to_string(),
        right_split: ui.right_split,
    };

    if let Err(e) = persist::save(&saved) {
        eprintln!("Warning: failed to save state: {}", e);
    }

    // Suppress rodio's "Dropping OutputStream" message:
    // 1. Redirect stderr to /dev/null, keeping a copy to restore afterwards
    // 2. Explicitly drop app (triggers AudioEngine → player thread shutdown)
    // 3. Brief sleep so the player thread can exit and drop OutputStream silently
    let saved_stderr = unsafe {
        let devnull = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);
        if devnull >= 0 {
            let saved = libc::dup(2);
            libc::dup2(devnull, 2);
            libc::close(devnull);
            saved
        } else {
            -1
        }
    };
    drop(app);
    std::thread::sleep(Duration::from_millis(50));
    // Without this every later stderr write is swallowed, including the error
    // reported by main and any panic message raised during teardown.
    if saved_stderr >= 0 {
        unsafe {
            libc::dup2(saved_stderr, 2);
            libc::close(saved_stderr);
        }
    }

    match draw_error {
        Some(e) => Err(e.into()),
        None => Ok(()),
    }
}

fn dirs_music_path() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        let music = PathBuf::from(home).join("Music");
        if music.is_dir() {
            return music;
        }
    }
    PathBuf::from(".")
}
