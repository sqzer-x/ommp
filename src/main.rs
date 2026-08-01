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

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    // Explicitly enable mouse motion tracking (SGR any-event mode)
    // Some terminals need this even after EnableMouseCapture
    stdout.write_all(b"\x1b[?1003h")?;
    stdout.flush()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    // Disable mouse motion tracking
    execute!(
        terminal.backend_mut(),
        crossterm::style::Print("\x1b[?1003l"),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

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
                                app.handle_action(app::AppAction::AddToQueue(all_indices));
                                ui.refresh_dir_browser(&app);

                                // Restore persisted state
                                if let Some(saved) = persist::load() {
                                    app.playback.volume = saved.volume.clamp(0.0, 1.0);
                                    app.playback.shuffle = saved.shuffle;
                                    app.playback.repeat = RepeatMode::from_label(&saved.repeat);
                                    app.handle_action(app::AppAction::SetVolume(app.playback.volume));
                                    ui.pane_widths = saved.pane_widths;
                                    ui.info_view = InfoView::from_label(&saved.info_view);
                                    ui.right_split = saved.right_split.clamp(10, 90);
                                    // Restore playlists (path → index remapping)
                                    let mut playlists = Vec::new();
                                    for sp in &saved.playlists {
                                        let tracks: Vec<usize> = sp.tracks.iter()
                                            .filter_map(|p| app.library.path_to_index(p))
                                            .collect();
                                        playlists.push(app::state::Playlist {
                                            name: sp.name.clone(),
                                            tracks,
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
                                    // Jump timeline to start of fade-out (1.5s mark)
                                    ui.splash_start = Some(
                                        std::time::Instant::now() - Duration::from_millis(1500)
                                    );
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
                        let size = terminal.size()?;
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
                        // Refresh hover + focus from stored mouse position
                        let size = terminal.size()?;
                        let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                        handler::refresh_hover(&app, &mut ui, area)
                    }
                    Event::LibraryReady(new_lib) => {
                        app.replace_library(new_lib);
                        ui.refresh_dir_browser(&app);
                        ui.clamp_selections(&app);
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
                            AudioEvent::TrackError(_) => {
                                // Skip to next track on decode error
                                vec![app::AppAction::NextTrack]
                            }
                            AudioEvent::Playing => {
                                app.playback.state = app::state::PlayState::Playing;
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
        terminal.draw(|frame| {
            ui.render(frame, &app);
        })?;
    }

    // Save state on exit
    let saved_playlists: Vec<persist::SavedPlaylist> = app.playlists.iter().map(|pl| {
        persist::SavedPlaylist {
            name: pl.name.clone(),
            tracks: pl.tracks.iter()
                .filter_map(|&idx| app.library.tracks.get(idx).map(|t| t.path.clone()))
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
    // 1. Redirect stderr to /dev/null
    // 2. Explicitly drop app (triggers AudioEngine → player thread shutdown)
    // 3. Brief sleep so the player thread can exit and drop OutputStream silently
    unsafe {
        let devnull = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);
        if devnull >= 0 {
            libc::dup2(devnull, 2);
            libc::close(devnull);
        }
    }
    drop(app);
    std::thread::sleep(Duration::from_millis(50));

    Ok(())
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
