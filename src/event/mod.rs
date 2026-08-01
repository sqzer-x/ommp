pub mod input;

#[derive(Debug)]
pub enum Event {
    Key(crossterm::event::KeyEvent),
    Mouse(crossterm::event::MouseEvent),
    /// Dimensions are not carried: the next draw re-queries the terminal.
    Resize,
    Tick,
    Audio(AudioEvent),
    LibraryReady(crate::library::Library),
}

#[derive(Debug, Clone)]
pub enum AudioEvent {
    PositionUpdate { position_secs: f64, duration_secs: f64 },
    TrackFinished,
    /// The current track could not be decoded.
    TrackError,
    /// The output device could not be opened. The player thread exits after
    /// sending this, so every later command is dropped — the UI has to say so
    /// rather than show a progress bar advancing over silence.
    DeviceError(String),
    Playing,
    Paused,
    Stopped,
}
