pub mod input;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
    Key(crossterm::event::KeyEvent),
    Mouse(crossterm::event::MouseEvent),
    Resize(u16, u16),
    Tick,
    Audio(AudioEvent),
    LibraryReady(crate::library::Library),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AudioEvent {
    PositionUpdate { position_secs: f64, duration_secs: f64 },
    TrackFinished,
    TrackError(String),
    /// The output device could not be opened. The player thread exits after
    /// sending this, so every later command is dropped — the UI has to say so
    /// rather than show a progress bar advancing over silence.
    DeviceError(String),
    Playing,
    Paused,
    Stopped,
}
