use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct SavedState {
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: String,
    pub pane_widths: [u16; 3],
    pub playlists: Vec<SavedPlaylist>,
    #[serde(default = "default_info_view")]
    pub info_view: String,
    #[serde(default = "default_right_split")]
    pub right_split: u16,
}

fn default_info_view() -> String {
    "Clock".to_string()
}

fn default_right_split() -> u16 {
    50
}

#[derive(Serialize, Deserialize)]
pub struct SavedPlaylist {
    pub name: String,
    pub tracks: Vec<PathBuf>,
}

fn state_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config/ommp/state.json")
}

pub fn save(state: &SavedState) -> anyhow::Result<()> {
    let path = state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state)?;
    fs::write(&path, json)?;
    Ok(())
}

pub fn load() -> Option<SavedState> {
    let path = state_path();
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}
