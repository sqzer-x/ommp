#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    Idle,
    Scanning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

impl RepeatMode {
    pub fn next(self) -> Self {
        match self {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RepeatMode::Off => "Off",
            RepeatMode::All => "All",
            RepeatMode::One => "One",
        }
    }

    pub fn from_label(s: &str) -> Self {
        match s {
            "All" => RepeatMode::All,
            "One" => RepeatMode::One,
            _ => RepeatMode::Off,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            RepeatMode::Off => "\u{F0456}",  // nf-md-repeat
            RepeatMode::All => "\u{F0456}",  // nf-md-repeat
            RepeatMode::One => "\u{F0458}",  // nf-md-repeat_once
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Queue,
    Directories,
    Artists,
    Albums,
    Genre,
    Format,
    Playlists,
}

impl Tab {
    pub const ALL: [Tab; 7] = [
        Tab::Queue,
        Tab::Directories,
        Tab::Artists,
        Tab::Albums,
        Tab::Genre,
        Tab::Format,
        Tab::Playlists,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Queue => "Queue",
            Tab::Directories => "Directories",
            Tab::Artists => "Artists",
            Tab::Albums => "Albums",
            Tab::Genre => "Genre",
            Tab::Format => "Format",
            Tab::Playlists => "Playlists",
        }
    }

    pub fn index(self) -> usize {
        Tab::ALL.iter().position(|&t| t == self).unwrap_or(0)
    }

    pub fn from_index(i: usize) -> Self {
        Tab::ALL.get(i).copied().unwrap_or(Tab::Queue)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    Library,
    Playlist,
    Lyrics,
}

impl FocusedPane {
    pub fn next(self) -> Self {
        match self {
            FocusedPane::Library => FocusedPane::Playlist,
            FocusedPane::Playlist => FocusedPane::Lyrics,
            FocusedPane::Lyrics => FocusedPane::Library,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            FocusedPane::Library => FocusedPane::Lyrics,
            FocusedPane::Playlist => FocusedPane::Library,
            FocusedPane::Lyrics => FocusedPane::Playlist,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub state: PlayState,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: RepeatMode,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            state: PlayState::Stopped,
            position_secs: 0.0,
            duration_secs: 0.0,
            volume: 0.8,
            shuffle: false,
            repeat: RepeatMode::Off,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct QueueState {
    pub tracks: Vec<usize>,
    pub current_index: Option<usize>,
    pub selected_index: usize,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub name: String,
    pub tracks: Vec<usize>,
}

impl Playlist {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tracks: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoView {
    Clock,
    AlbumArt,
}

impl InfoView {
    pub fn next(self) -> Self {
        match self {
            InfoView::Clock => InfoView::AlbumArt,
            InfoView::AlbumArt => InfoView::Clock,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            InfoView::Clock => "Clock",
            InfoView::AlbumArt => "AlbumArt",
        }
    }

    pub fn from_label(s: &str) -> Self {
        match s {
            "AlbumArt" => InfoView::AlbumArt,
            _ => InfoView::Clock,
        }
    }
}

