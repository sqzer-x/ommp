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

/// Give up after this many undecodable files in a row rather than walking the
/// whole queue at decoder speed.
const MAX_CONSECUTIVE_FAILURES: u32 = 5;

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
    /// Append to the queue, leaving what is playing alone.
    AddToQueue(Vec<usize>),
    /// Discard the queue and start over from the given tracks.
    ReplaceQueue(Vec<usize>),
    ClearQueue,
    RemoveFromQueue(usize),
    PlayQueueIndex(usize),
    UpdatePosition { position_secs: f64, duration_secs: f64 },
    TrackFinished,
    /// The engine could not decode the current track.
    TrackFailed,
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
    /// Set when the output device could not be opened; nothing will ever play.
    pub audio_error: Option<String>,
    /// Undecodable files in a row. Bounded so a queue of broken files stops
    /// instead of spinning through them forever.
    consecutive_failures: u32,
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
            audio_error: None,
            consecutive_failures: 0,
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
                self.advance(true);
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
                // Anchor on the current track so turning shuffle on mid-song
                // doesn't jump, and so "previous" has somewhere to go back to.
                self.rebuild_shuffle_order(true);
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
                let was_empty = self.queue.tracks.is_empty();
                self.queue.tracks.extend(track_indices);
                // Only take over the current position when there was nothing to
                // disturb — appending must not interrupt what is playing.
                if was_empty && !self.queue.tracks.is_empty() {
                    self.queue.current_index = Some(0);
                }
                self.resync_shuffle_order(true);
            }
            AppAction::ReplaceQueue(track_indices) => {
                self.queue.tracks = track_indices;
                self.queue.current_index =
                    if self.queue.tracks.is_empty() { None } else { Some(0) };
                self.queue.selected_index = 0;
                self.queue.scroll_offset = 0;
                self.resync_shuffle_order(false);
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
                if idx >= self.queue.tracks.len() {
                    return;
                }
                let removed_current = self.queue.current_index == Some(idx);
                self.queue.tracks.remove(idx);

                if self.queue.tracks.is_empty() {
                    self.queue.current_index = None;
                    self.stop_playback();
                } else if let Some(ci) = self.queue.current_index {
                    if idx < ci {
                        self.queue.current_index = Some(ci - 1);
                    } else if removed_current {
                        // The removed track is still audible. Leaving
                        // current_index alone would point it at whatever slid
                        // into the slot, and that track would then be skipped.
                        let next = ci.min(self.queue.tracks.len() - 1);
                        if !self.play_queue_position(next) {
                            self.stop_playback();
                        }
                    }
                }
                // Never leave the selection past the end: the highlight
                // disappears and both `j` and `d` stop responding.
                self.queue.selected_index = self
                    .queue
                    .selected_index
                    .min(self.queue.tracks.len().saturating_sub(1));
                self.resync_shuffle_order(true);
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
                self.advance(false);
            }
            AppAction::TrackFailed => {
                self.consecutive_failures += 1;
                if self.consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                    self.consecutive_failures = 0;
                    self.stop_playback();
                } else {
                    // Advance as though the user asked: retrying under
                    // RepeatMode::One would re-open the same broken file forever
                    // and flood the event channel.
                    self.advance(true);
                }
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
        // The playing file is gone from the library. Without stopping, audio
        // carries on while current_track() returns None, so the status bar,
        // album art and info pane all blank out mid-song.
        if new_current.is_none() && self.queue.current_index.is_some() {
            self.stop_playback();
        }
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
        self.resync_shuffle_order(true);
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
        // Keep the shuffled pass pointing at what is actually playing, so a
        // double-clicked row becomes the new starting point for next/previous.
        if let Some(p) = self.queue.shuffle_order.iter().position(|&q| q == qi) {
            self.queue.shuffle_pos = p;
        }
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

    /// Draw a fresh permutation of the queue positions. With `anchor_current`
    /// the playing track is moved to the front so toggling shuffle mid-song does
    /// not jump; on a repeat-all wrap it is not, so the new pass starts clean.
    fn rebuild_shuffle_order(&mut self, anchor_current: bool) {
        if !self.playback.shuffle {
            self.queue.shuffle_order.clear();
            self.queue.shuffle_pos = 0;
            return;
        }

        use rand::seq::SliceRandom;
        let mut order: Vec<usize> = (0..self.queue.tracks.len()).collect();
        order.shuffle(&mut rand::thread_rng());

        if anchor_current {
            if let Some(cur) = self.queue.current_index {
                if let Some(p) = order.iter().position(|&q| q == cur) {
                    order.swap(0, p);
                }
            }
        }

        self.queue.shuffle_order = order;
        self.queue.shuffle_pos = 0;
    }

    /// Rebuild only when the queue changed shape underneath the order.
    fn resync_shuffle_order(&mut self, anchor_current: bool) {
        if self.playback.shuffle && self.queue.shuffle_order.len() != self.queue.tracks.len() {
            self.rebuild_shuffle_order(anchor_current);
        }
    }

    /// Next position in the shuffled pass, or None once the pass is exhausted
    /// and repeat is not All. Returning None is what lets a shuffled queue with
    /// repeat off actually finish.
    fn shuffle_next(&mut self) -> Option<usize> {
        self.resync_shuffle_order(true);
        let next_pos = self.queue.shuffle_pos + 1;
        if next_pos < self.queue.shuffle_order.len() {
            self.queue.shuffle_pos = next_pos;
        } else if self.playback.repeat == RepeatMode::All {
            self.rebuild_shuffle_order(false);
        } else {
            return None;
        }
        self.queue.shuffle_order.get(self.queue.shuffle_pos).copied()
    }

    fn shuffle_prev(&mut self) -> Option<usize> {
        self.resync_shuffle_order(true);
        self.queue.shuffle_pos = self.queue.shuffle_pos.saturating_sub(1);
        self.queue.shuffle_order.get(self.queue.shuffle_pos).copied()
    }

    /// Move to the next track. `user_initiated` separates pressing `n` from a
    /// track ending: RepeatMode::One must repeat on the latter and advance on
    /// the former, otherwise the skip key does nothing while repeat-one is on.
    fn advance(&mut self, user_initiated: bool) {
        if self.queue.tracks.is_empty() {
            return;
        }

        if !user_initiated && self.playback.repeat == RepeatMode::One {
            let current = self.queue.current_index;
            match current {
                Some(idx) if self.play_queue_position(idx) => {}
                _ => self.stop_playback(),
            }
            return;
        }

        let next = if self.playback.shuffle {
            self.shuffle_next()
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

        // Under shuffle, walk the shuffled pass backwards. Stepping to
        // current_index - 1 would land on a queue row that was never played.
        let prev = if self.playback.shuffle {
            self.shuffle_prev()
        } else if let Some(idx) = self.queue.current_index {
            Some(if idx > 0 {
                idx - 1
            } else if self.playback.repeat == RepeatMode::All {
                self.queue.tracks.len() - 1
            } else {
                0
            })
        } else {
            Some(0)
        };

        match prev {
            Some(prev_idx) if self.play_queue_position(prev_idx) => {}
            _ => self.stop_playback(),
        }
    }

    /// The engine confirmed audio is running, so the failure streak is over.
    pub fn note_playback_started(&mut self) {
        self.consecutive_failures = 0;
    }

    pub fn current_track(&self) -> Option<&crate::library::track::Track> {
        self.queue
            .current_index
            .and_then(|qi| self.queue.tracks.get(qi))
            .and_then(|&ti| self.library.tracks.get(ti))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A queue over `len` resolvable library tracks. There is no audio engine,
    /// so `PlayerCommand`s go nowhere, but `play_queue_position` succeeds and
    /// every queue and shuffle transition is observable.
    fn app_with_queue(len: usize) -> App {
        let mut app = App::new(PathBuf::from("."));
        app.library.tracks = (0..len)
            .map(|i| crate::library::track::Track {
                path: PathBuf::from(format!("/music/{i}.flac")),
                title: format!("track {i}"),
                artist: String::new(),
                album: String::new(),
                album_artist: String::new(),
                genre: String::new(),
                year: None,
                track_number: None,
                track_total: None,
                disc_number: None,
                disc_total: None,
                comment: None,
                duration: std::time::Duration::from_secs(1),
                bitrate: None,
                sample_rate: None,
                bit_depth: None,
                channels: None,
                lyrics: None,
            })
            .collect();
        app.handle_action(AppAction::ReplaceQueue((0..len).collect()));
        app
    }

    #[test]
    fn add_to_queue_appends_instead_of_replacing() {
        let mut app = app_with_queue(3);
        app.queue.current_index = Some(1);

        app.handle_action(AppAction::AddToQueue(vec![7, 8]));

        assert_eq!(app.queue.tracks, vec![0, 1, 2, 7, 8]);
        // Appending must leave what is playing alone.
        assert_eq!(app.queue.current_index, Some(1));
    }

    #[test]
    fn add_to_an_empty_queue_takes_the_first_slot() {
        let mut app = app_with_queue(0);
        assert_eq!(app.queue.current_index, None);

        app.handle_action(AppAction::AddToQueue(vec![4, 5]));

        assert_eq!(app.queue.current_index, Some(0));
    }

    #[test]
    fn removing_an_entry_before_the_current_one_shifts_it_down() {
        let mut app = app_with_queue(4);
        app.queue.current_index = Some(2);

        app.handle_action(AppAction::RemoveFromQueue(0));

        assert_eq!(app.queue.tracks, vec![1, 2, 3]);
        assert_eq!(app.queue.current_index, Some(1));
    }

    #[test]
    fn removing_the_last_row_pulls_the_selection_back_in_range() {
        let mut app = app_with_queue(3);
        app.queue.selected_index = 2;

        app.handle_action(AppAction::RemoveFromQueue(2));

        // Left past the end, the highlight vanishes and j/d both stop working.
        assert_eq!(app.queue.selected_index, 1);
        assert!(app.queue.selected_index < app.queue.tracks.len());
    }

    #[test]
    fn clearing_the_queue_stops_rather_than_leaving_playback_running() {
        let mut app = app_with_queue(3);
        app.playback.state = PlayState::Playing;

        app.handle_action(AppAction::ClearQueue);

        assert_eq!(app.playback.state, PlayState::Stopped);
        assert_eq!(app.queue.current_index, None);
    }

    #[test]
    fn shuffle_order_is_a_permutation_anchored_on_the_current_track() {
        let mut app = app_with_queue(20);
        app.queue.current_index = Some(7);

        app.handle_action(AppAction::ToggleShuffle);

        assert!(app.playback.shuffle);
        assert_eq!(app.queue.shuffle_order.len(), 20);
        let mut sorted = app.queue.shuffle_order.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..20).collect::<Vec<_>>());
        // Anchored, so turning shuffle on mid-song does not jump.
        assert_eq!(app.queue.shuffle_order[0], 7);
    }

    #[test]
    fn a_shuffled_pass_visits_every_track_once_and_then_ends() {
        let mut app = app_with_queue(6);
        app.handle_action(AppAction::ToggleShuffle);

        let mut seen = vec![app.queue.shuffle_order[0]];
        while let Some(next) = app.shuffle_next() {
            seen.push(next);
        }

        seen.sort_unstable();
        // Every track exactly once — the old random pick repeated some and
        // skipped others, and never returned None so repeat-off never stopped.
        assert_eq!(seen, (0..6).collect::<Vec<_>>());
    }

    #[test]
    fn shuffle_prev_walks_back_along_the_shuffled_pass() {
        let mut app = app_with_queue(5);
        app.handle_action(AppAction::ToggleShuffle);
        let order = app.queue.shuffle_order.clone();

        assert_eq!(app.shuffle_next(), Some(order[1]));
        assert_eq!(app.shuffle_next(), Some(order[2]));
        // Not order[1] - 1, and not current_index - 1.
        assert_eq!(app.shuffle_prev(), Some(order[1]));
        assert_eq!(app.shuffle_prev(), Some(order[0]));
        // Clamps at the start of the pass rather than underflowing.
        assert_eq!(app.shuffle_prev(), Some(order[0]));
    }

    #[test]
    fn repeat_one_repeats_when_a_track_ends_but_not_when_the_user_skips() {
        let mut app = app_with_queue(3);
        app.playback.repeat = RepeatMode::One;
        app.queue.current_index = Some(0);

        // Track ended on its own: repeat it.
        app.advance(false);
        assert_eq!(app.queue.current_index, Some(0));

        // User pressed `n`: move on. Repeating here left the skip key dead.
        app.advance(true);
        assert_eq!(app.queue.current_index, Some(1));
    }

    #[test]
    fn repeat_off_stops_at_the_end_of_the_queue() {
        let mut app = app_with_queue(2);
        app.queue.current_index = Some(1);

        app.advance(false);

        assert_eq!(app.playback.state, PlayState::Stopped);
    }

    #[test]
    fn shuffle_with_repeat_off_stops_once_the_pass_is_done() {
        let mut app = app_with_queue(4);
        app.handle_action(AppAction::ToggleShuffle);

        // One advance per remaining track, then one more.
        for _ in 0..4 {
            app.advance(false);
        }

        // The old random pick always returned Some, so this never terminated.
        assert_eq!(app.playback.state, PlayState::Stopped);
    }

    #[test]
    fn removing_the_playing_entry_moves_on_instead_of_skipping_a_track() {
        let mut app = app_with_queue(3);
        app.queue.current_index = Some(0);

        app.handle_action(AppAction::RemoveFromQueue(0));

        assert_eq!(app.queue.tracks, vec![1, 2]);
        // Position 0 now holds what used to be track 1; it must actually start,
        // not merely be pointed at while the removed track keeps playing.
        assert_eq!(app.queue.current_index, Some(0));
        assert_eq!(app.playback.state, PlayState::Playing);
    }

    #[test]
    fn repeated_decode_failures_give_up_instead_of_looping() {
        let mut app = app_with_queue(3);
        app.playback.repeat = RepeatMode::One;
        app.queue.current_index = Some(0);

        // Under repeat-one this used to re-open the same broken file forever.
        for _ in 0..MAX_CONSECUTIVE_FAILURES {
            app.handle_action(AppAction::TrackFailed);
        }

        assert_eq!(app.playback.state, PlayState::Stopped);
    }
}
