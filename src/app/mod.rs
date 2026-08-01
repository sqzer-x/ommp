pub mod handler;
pub mod persist;
pub mod state;

use std::collections::HashMap;
use std::path::PathBuf;

use crossbeam_channel::Sender;

use crate::audio::{AudioEngine, PlayerCommand};
use crate::event::Event;
use crate::library::Library;
use state::*;

#[derive(Debug, Clone)]
pub enum AppAction {
    Quit,
    PauseResume,
    NextTrack,
    PrevTrack,
    SetVolume(f32),
    VolumeUp,
    VolumeDown,
    Seek(f64),
    SeekForward,
    SeekBackward,
    ToggleShuffle,
    CycleRepeat,
    SwitchTab(Tab),
    FocusNext,
    FocusPrev,
    FocusPane(FocusedPane),
    AddToQueue(Vec<usize>),
    ClearQueue,
    RemoveFromQueue(usize),
    PlayQueueIndex(usize),
    UpdatePosition { position_secs: f64, duration_secs: f64 },
    TrackFinished,
    SetQueueSelection(usize),
    AddToPlaylist { playlist_idx: usize, track_idx: usize },
    RemoveFromPlaylist { playlist_idx: usize, track_idx: usize },
    CreatePlaylist(String),
    DeletePlaylist(usize),
    RenamePlaylist { idx: usize, name: String },
    LibrarySync,
}

pub struct App {
    pub should_quit: bool,
    pub tab: Tab,
    pub focus: FocusedPane,
    pub playback: PlaybackState,
    pub queue: QueueState,
    pub library: Library,
    pub music_dir: PathBuf,
    pub search_query: String,
    pub search_mode: bool,
    pub search_results: Vec<usize>,
    pub playlists: Vec<state::Playlist>,
    pub sync_state: SyncState,
    pub initial_scan_complete: bool,
    audio_engine: Option<AudioEngine>,
    event_tx: Option<Sender<Event>>,
}

impl App {
    pub fn new(music_dir: PathBuf) -> Self {
        Self {
            should_quit: false,
            tab: Tab::Queue,
            focus: FocusedPane::Library,
            playback: PlaybackState::default(),
            queue: QueueState::default(),
            library: Library::new(),
            music_dir,
            search_query: String::new(),
            search_mode: false,
            search_results: Vec::new(),
            playlists: vec![state::Playlist::new("Bookmarks")],
            sync_state: SyncState::Idle,
            initial_scan_complete: false,
            audio_engine: None,
            event_tx: None,
        }
    }

    pub fn set_event_tx(&mut self, tx: Sender<Event>) {
        self.event_tx = Some(tx);
    }

    pub fn set_audio_engine(&mut self, engine: AudioEngine) {
        self.audio_engine = Some(engine);
    }

    pub fn handle_action(&mut self, action: AppAction) {
        match action {
            AppAction::Quit => {
                self.should_quit = true;
                if let Some(ref engine) = self.audio_engine {
                    engine.send(PlayerCommand::Stop);
                }
            }
            AppAction::PauseResume => match self.playback.state {
                PlayState::Playing => {
                    if let Some(ref engine) = self.audio_engine {
                        engine.send(PlayerCommand::Pause);
                    }
                    self.playback.state = PlayState::Paused;
                }
                PlayState::Paused => {
                    if let Some(ref engine) = self.audio_engine {
                        engine.send(PlayerCommand::Resume);
                    }
                    self.playback.state = PlayState::Playing;
                }
                PlayState::Stopped => {
                    // Try to play current queue item
                    if let Some(idx) = self.queue.current_index {
                        self.play_queue_position(idx);
                    }
                }
            },
            AppAction::NextTrack => {
                self.play_next();
            }
            AppAction::PrevTrack => {
                self.play_prev();
            }
            AppAction::SetVolume(vol) => {
                self.playback.volume = vol.clamp(0.0, 1.0);
                if let Some(ref engine) = self.audio_engine {
                    engine.send(PlayerCommand::SetVolume(self.playback.volume));
                }
            }
            AppAction::VolumeUp => {
                let vol = (self.playback.volume + 0.05).min(1.0);
                self.handle_action(AppAction::SetVolume(vol));
            }
            AppAction::VolumeDown => {
                let vol = (self.playback.volume - 0.05).max(0.0);
                self.handle_action(AppAction::SetVolume(vol));
            }
            AppAction::Seek(secs) => {
                let clamped = secs.clamp(0.0, self.playback.duration_secs);
                if let Some(ref engine) = self.audio_engine {
                    engine.send(PlayerCommand::Seek(clamped));
                }
                self.playback.position_secs = clamped;
            }
            AppAction::SeekForward => {
                let pos = self.playback.position_secs + 5.0;
                self.handle_action(AppAction::Seek(pos));
            }
            AppAction::SeekBackward => {
                let pos = self.playback.position_secs - 5.0;
                self.handle_action(AppAction::Seek(pos));
            }
            AppAction::ToggleShuffle => {
                self.playback.shuffle = !self.playback.shuffle;
            }
            AppAction::CycleRepeat => {
                self.playback.repeat = self.playback.repeat.next();
            }
            AppAction::SwitchTab(tab) => {
                self.tab = tab;
            }
            AppAction::FocusNext => {
                self.focus = self.focus.next();
            }
            AppAction::FocusPrev => {
                self.focus = self.focus.prev();
            }
            AppAction::FocusPane(pane) => {
                self.focus = pane;
            }
            AppAction::AddToQueue(track_indices) => {
                self.queue.tracks = track_indices;
                self.queue.current_index = if self.queue.tracks.is_empty() { None } else { Some(0) };
                self.queue.selected_index = 0;
                self.queue.scroll_offset = 0;
            }
            AppAction::ClearQueue => {
                self.queue.tracks.clear();
                self.queue.current_index = None;
                self.queue.selected_index = 0;
                self.queue.scroll_offset = 0;
                // Without this the engine keeps playing a track the queue no
                // longer contains, and its position updates keep the clock running.
                self.stop_playback();
            }
            AppAction::RemoveFromQueue(idx) => {
                if idx < self.queue.tracks.len() {
                    self.queue.tracks.remove(idx);
                    if self.queue.tracks.is_empty() {
                        self.queue.current_index = None;
                    } else if let Some(ref mut ci) = self.queue.current_index {
                        if idx < *ci {
                            *ci -= 1;
                        } else if idx == *ci && *ci >= self.queue.tracks.len() {
                            *ci = self.queue.tracks.len() - 1;
                        }
                    }
                }
            }
            AppAction::PlayQueueIndex(idx) => {
                // play_queue_position only commits current_index once the track
                // is known to resolve, so a stale row cannot move the ▶ marker
                // while a different track keeps playing.
                self.play_queue_position(idx);
            }
            AppAction::UpdatePosition { position_secs, duration_secs } => {
                self.playback.position_secs = position_secs;
                if duration_secs > 0.0 {
                    self.playback.duration_secs = duration_secs;
                }
            }
            AppAction::TrackFinished => {
                self.play_next();
            }
            AppAction::SetQueueSelection(idx) => {
                if idx < self.queue.tracks.len() {
                    self.queue.selected_index = idx;
                }
            }
            AppAction::AddToPlaylist { playlist_idx, track_idx } => {
                if let Some(pl) = self.playlists.get_mut(playlist_idx) {
                    if !pl.tracks.contains(&track_idx) {
                        pl.tracks.push(track_idx);
                    }
                }
            }
            AppAction::RemoveFromPlaylist { playlist_idx, track_idx } => {
                if let Some(pl) = self.playlists.get_mut(playlist_idx) {
                    pl.tracks.retain(|&t| t != track_idx);
                }
            }
            AppAction::CreatePlaylist(name) => {
                self.playlists.push(state::Playlist::new(name));
            }
            AppAction::DeletePlaylist(idx) => {
                if idx < self.playlists.len() {
                    self.playlists.remove(idx);
                }
            }
            AppAction::RenamePlaylist { idx, name } => {
                if let Some(pl) = self.playlists.get_mut(idx) {
                    pl.name = name;
                }
            }
            AppAction::LibrarySync => {
                if self.sync_state == SyncState::Scanning || !self.initial_scan_complete {
                    return;
                }
                self.sync_state = SyncState::Scanning;
                if let Some(ref tx) = self.event_tx {
                    let dir = self.music_dir.clone();
                    let tx = tx.clone();
                    std::thread::spawn(move || {
                        let lib = Library::scan(&dir);
                        let _ = tx.send(Event::LibraryReady(lib));
                    });
                }
            }
        }
    }

    pub fn replace_library(&mut self, new_lib: Library) {
        // An empty scan against a non-empty library means the music directory
        // went away (unmounted, renamed, permissions), not that the user deleted
        // every file. Applying it would prune the queue and every playlist, and
        // the pruned result is what gets written to disk on exit.
        if new_lib.tracks.is_empty() && !self.library.tracks.is_empty() {
            self.sync_state = SyncState::Idle;
            return;
        }

        // Build path→new_index map
        let path_map: HashMap<PathBuf, usize> = new_lib.tracks.iter().enumerate()
            .map(|(i, t)| (t.path.clone(), i))
            .collect();

        // Capture current playing track path
        let playing_path = self.queue.current_index
            .and_then(|qi| self.queue.tracks.get(qi))
            .and_then(|&ti| self.library.tracks.get(ti))
            .map(|t| t.path.clone());

        // Remap queue tracks
        let new_queue_tracks: Vec<usize> = self.queue.tracks.iter()
            .filter_map(|&old_idx| {
                self.library.tracks.get(old_idx)
                    .and_then(|t| path_map.get(&t.path))
                    .copied()
            })
            .collect();

        // Remap current_index: find playing track in new queue
        let new_current = playing_path.and_then(|pp| {
            path_map.get(&pp).and_then(|&new_ti| {
                new_queue_tracks.iter().position(|&idx| idx == new_ti)
            })
        });

        self.queue.tracks = new_queue_tracks;
        self.queue.current_index = new_current;
        self.queue.selected_index = self.queue.selected_index.min(
            self.queue.tracks.len().saturating_sub(1)
        );
        self.queue.scroll_offset = self.queue.scroll_offset.min(
            self.queue.tracks.len().saturating_sub(1)
        );

        // Remap playlists. Entries the new library lacks are parked in `missing`
        // rather than dropped, and anything parked earlier is restored if it came
        // back.
        for pl in &mut self.playlists {
            let mut tracks = Vec::with_capacity(pl.tracks.len());
            let mut missing = Vec::new();
            for &old_idx in &pl.tracks {
                let Some(path) = self.library.tracks.get(old_idx).map(|t| &t.path) else {
                    continue;
                };
                match path_map.get(path) {
                    Some(&new_idx) => tracks.push(new_idx),
                    None => missing.push(path.clone()),
                }
            }
            for path in pl.missing.drain(..) {
                match path_map.get(&path) {
                    Some(&new_idx) => tracks.push(new_idx),
                    None => missing.push(path),
                }
            }
            pl.tracks = tracks;
            pl.missing = missing;
        }

        // Remap search results
        if !self.search_query.is_empty() {
            self.search_results = new_lib.search(&self.search_query);
        }

        self.library = new_lib;
        self.sync_state = SyncState::Idle;
    }

    /// Start the track at queue position `qi`, the single place playback is
    /// actually kicked off. Returns false when the position — or the library
    /// index behind it — no longer exists, which is why every caller goes
    /// through here instead of indexing `library.tracks` directly.
    fn play_queue_position(&mut self, qi: usize) -> bool {
        let Some((path, dur)) = self
            .queue
            .tracks
            .get(qi)
            .and_then(|&ti| self.library.tracks.get(ti))
            .map(|t| (t.path.clone(), t.duration.as_secs_f64()))
        else {
            return false;
        };

        if let Some(ref engine) = self.audio_engine {
            engine.send(PlayerCommand::Play(path));
        }
        self.queue.current_index = Some(qi);
        self.playback.state = PlayState::Playing;
        self.playback.position_secs = 0.0;
        self.playback.duration_secs = dur;
        true
    }

    /// Tell the audio engine to stop and mark playback stopped. Setting only the
    /// UI state leaves the engine playing and its position updates overwriting
    /// the clock.
    fn stop_playback(&mut self) {
        if let Some(ref engine) = self.audio_engine {
            engine.send(PlayerCommand::Stop);
        }
        self.playback.state = PlayState::Stopped;
        self.playback.position_secs = 0.0;
    }

    fn play_next(&mut self) {
        if self.queue.tracks.is_empty() {
            return;
        }

        match self.playback.repeat {
            RepeatMode::One => {
                if let Some(idx) = self.queue.current_index {
                    self.play_queue_position(idx);
                }
            }
            _ => {
                let next = if self.playback.shuffle {
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    Some(rng.gen_range(0..self.queue.tracks.len()))
                } else if let Some(idx) = self.queue.current_index {
                    let next_idx = idx + 1;
                    if next_idx < self.queue.tracks.len() {
                        Some(next_idx)
                    } else if self.playback.repeat == RepeatMode::All {
                        Some(0)
                    } else {
                        None
                    }
                } else {
                    Some(0)
                };

                match next {
                    Some(next_idx) if self.play_queue_position(next_idx) => {}
                    _ => self.stop_playback(),
                }
            }
        }
    }

    fn play_prev(&mut self) {
        if self.queue.tracks.is_empty() {
            return;
        }

        // If more than 3 seconds in, restart current track
        if self.playback.position_secs > 3.0 {
            if let Some(idx) = self.queue.current_index {
                if self.play_queue_position(idx) {
                    return;
                }
            }
        }

        let prev = if let Some(idx) = self.queue.current_index {
            if idx > 0 {
                idx - 1
            } else if self.playback.repeat == RepeatMode::All {
                self.queue.tracks.len() - 1
            } else {
                0
            }
        } else {
            0
        };

        if !self.play_queue_position(prev) {
            self.stop_playback();
        }
    }

    pub fn current_track(&self) -> Option<&crate::library::track::Track> {
        self.queue
            .current_index
            .and_then(|qi| self.queue.tracks.get(qi))
            .and_then(|&ti| self.library.tracks.get(ti))
    }
}
