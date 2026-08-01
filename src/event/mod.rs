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
    Playing,
    Paused,
    Stopped,
}
