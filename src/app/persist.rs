use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Every field defaults, so a state file written by an older build — or one
/// missing a key — restores what it can instead of dropping the whole session.
#[derive(Serialize, Deserialize)]
pub struct SavedState {
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub shuffle: bool,
    #[serde(default = "default_repeat")]
    pub repeat: String,
    #[serde(default = "default_pane_widths")]
    pub pane_widths: [u16; 3],
    #[serde(default)]
    pub playlists: Vec<SavedPlaylist>,
    #[serde(default = "default_info_view")]
    pub info_view: String,
    #[serde(default = "default_right_split")]
    pub right_split: u16,
}

fn default_volume() -> f32 {
    0.5
}

fn default_repeat() -> String {
    "Off".to_string()
}

pub const DEFAULT_PANE_WIDTHS: [u16; 3] = [20, 60, 20];

fn default_pane_widths() -> [u16; 3] {
    DEFAULT_PANE_WIDTHS
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

/// Pane widths must each clear the 10% minimum the resize code assumes and sum
/// to exactly 100; anything else makes `LayoutAreas` produce degenerate rects
/// and the border-drag math underflow. Reject rather than try to repair.
pub fn sane_pane_widths(w: [u16; 3]) -> [u16; 3] {
    if w.iter().all(|&p| p >= 10) && w.iter().sum::<u16>() == 100 {
        w
    } else {
        DEFAULT_PANE_WIDTHS
    }
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
    // Write-then-rename: a crash or full disk mid-write leaves the previous
    // state file intact instead of a truncated one that parses to nothing.
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn load() -> Option<SavedState> {
    let path = state_path();
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_widths_reject_anything_layout_cannot_use() {
        assert_eq!(sane_pane_widths([20, 60, 20]), [20, 60, 20]);
        assert_eq!(sane_pane_widths([10, 80, 10]), [10, 80, 10]);
        // Below the 10% floor the resize math underflows.
        assert_eq!(sane_pane_widths([10, 5, 85]), DEFAULT_PANE_WIDTHS);
        // Sums that are not 100 leave the dashboard with a gap or overflow.
        assert_eq!(sane_pane_widths([20, 20, 20]), DEFAULT_PANE_WIDTHS);
        assert_eq!(sane_pane_widths([0, 0, 0]), DEFAULT_PANE_WIDTHS);
    }

    #[test]
    fn missing_fields_fall_back_instead_of_dropping_the_file() {
        let saved: SavedState = serde_json::from_str(r#"{"volume":0.25}"#).unwrap();
        assert_eq!(saved.volume, 0.25);
        assert_eq!(saved.repeat, "Off");
        assert_eq!(saved.pane_widths, DEFAULT_PANE_WIDTHS);
        assert_eq!(saved.right_split, 50);
        assert!(saved.playlists.is_empty());
    }
}
