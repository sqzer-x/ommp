use rodio::Decoder;
use std::fs::File;
use std::io::BufReader;
use std::panic;
use std::path::Path;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use walkdir::WalkDir;

use super::track::Track;

const AUDIO_EXTENSIONS: &[&str] = &["flac", "mp3", "m4a", "ogg", "wav", "opus", "aac", "wma"];

/// Check if we can decode this file: try rodio first, then symphonia direct probe
fn is_decodable(path: &Path) -> bool {
    let path = path.to_path_buf();
    let result = panic::catch_unwind(move || {
        // Try rodio auto-detect
        if let Ok(file) = File::open(&path) {
            if Decoder::new(BufReader::new(file)).is_ok() {
                return true;
            }
        }

        // Try symphonia direct probe (handles M4A/ALAC/MP4 that rodio can't)
        if let Ok(file) = File::open(&path) {
            let mss = MediaSourceStream::new(Box::new(file), Default::default());
            let mut hint = Hint::new();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                hint.with_extension(ext);
            }
            if symphonia::default::get_probe()
                .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
                .is_ok()
            {
                return true;
            }
        }

        false
    });
    result.unwrap_or(false)
}

pub fn scan_directory(path: &Path) -> Vec<Track> {
    let mut tracks = Vec::new();

    for entry in WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // Skip macOS resource fork files
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("._") {
                continue;
            }
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        if let Some(ext) = ext {
            if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
                if !is_decodable(path) {
                    continue;
                }
                if let Some(track) = Track::from_path(path) {
                    tracks.push(track);
                }
            }
        }
    }

    tracks.sort_by(|a, b| {
        a.album_artist
            .cmp(&b.album_artist)
            .then(a.album.cmp(&b.album))
            .then(a.track_number.cmp(&b.track_number))
            .then(a.title.cmp(&b.title))
    });

    tracks
}
