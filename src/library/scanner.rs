use rodio::Decoder;
use std::fs::File;
use std::io::BufReader;
use std::panic;
use std::path::{Path, PathBuf};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use walkdir::WalkDir;

use super::cache::{self, Entry};
use super::track::Track;

const AUDIO_EXTENSIONS: &[&str] = &["flac", "mp3", "m4a", "ogg", "wav", "opus", "aac", "wma"];

/// A candidate file and the stamp that decides whether the cache still applies.
struct Candidate {
    path: PathBuf,
    stamp: (u64, u64),
}

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

/// Walk the tree and list audio files with their stamps. Only `stat` here —
/// walkdir has already done it, so this costs nothing extra.
fn collect_candidates(root: &Path) -> Vec<Candidate> {
    WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let path = entry.path();

            // Skip macOS resource fork files
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("._"))
            {
                return None;
            }

            let ext = path.extension().and_then(|e| e.to_str())?.to_lowercase();
            if !AUDIO_EXTENSIONS.contains(&ext.as_str()) {
                return None;
            }

            let meta = entry.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            Some(Candidate { path: path.to_path_buf(), stamp: cache::stamp(&meta) })
        })
        .collect()
}

fn parse_one(candidate: &Candidate) -> Option<Entry> {
    if !is_decodable(&candidate.path) {
        return None;
    }
    Track::from_path(&candidate.path).map(|track| Entry {
        track,
        mtime: candidate.stamp.0,
        size: candidate.stamp.1,
    })
}

/// Parse across all cores. Reading a file is mostly waiting on the disk and
/// then parsing a header, both of which overlap well; scanning was serial, so a
/// large library paid the full per-file cost one file at a time.
fn parse_all(candidates: &[Candidate]) -> Vec<Entry> {
    if candidates.is_empty() {
        return Vec::new();
    }

    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(candidates.len());
    let chunk = candidates.len().div_ceil(workers);

    std::thread::scope(|scope| {
        let handles: Vec<_> = candidates
            .chunks(chunk)
            .map(|c| scope.spawn(move || c.iter().filter_map(parse_one).collect::<Vec<Entry>>()))
            .collect();
        handles
            .into_iter()
            // A panicking worker loses its chunk rather than the whole scan.
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    })
}

pub fn scan_directory(path: &Path) -> Vec<Track> {
    let candidates = collect_candidates(path);
    let mut cached = cache::load();

    // Reuse whatever still matches on disk; parse the rest.
    let mut entries: Vec<Entry> = Vec::with_capacity(candidates.len());
    let mut stale: Vec<Candidate> = Vec::new();
    for candidate in candidates {
        match cached.remove(&candidate.path) {
            Some(entry) if cache::is_fresh(&entry, candidate.stamp) => entries.push(entry),
            _ => stale.push(candidate),
        }
    }
    entries.extend(parse_all(&stale));

    entries.sort_by(|a, b| {
        a.track
            .album_artist
            .cmp(&b.track.album_artist)
            .then(a.track.album.cmp(&b.track.album))
            .then(a.track.track_number.cmp(&b.track.track_number))
            .then(a.track.title.cmp(&b.track.title))
    });

    cache::save(&entries);
    entries.into_iter().map(|e| e.track).collect()
}
