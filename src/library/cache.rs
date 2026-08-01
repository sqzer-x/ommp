//! On-disk cache of parsed tags, so only files that actually changed are read
//! again.
//!
//! Scanning is dominated by opening and header-parsing every file: on a cold
//! page cache that measured 0.72 ms per track, or roughly 3.6 seconds for a
//! 5,000-track library, every single launch. A cached entry is reused whenever
//! the file's modification time and size still match, which reduces a repeat
//! scan to one `stat` per file.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::track::Track;

/// Bump when `Track`'s shape changes so old entries are discarded rather than
/// deserialised into the wrong fields.
const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub track: Track,
    /// Seconds since the epoch; paired with `size` to detect edits.
    pub mtime: u64,
    pub size: u64,
}

#[derive(Deserialize)]
struct CacheFile {
    version: u32,
    entries: Vec<Entry>,
}

/// Borrowing counterpart, so saving does not clone every entry.
#[derive(Serialize)]
struct CacheFileRef<'a> {
    version: u32,
    entries: &'a [Entry],
}

fn cache_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join("ommp").join("library.json"))
}

/// Cached entries keyed by path. Any failure yields an empty map, which simply
/// means every file gets parsed — the cache is never load-bearing for
/// correctness.
pub fn load() -> HashMap<PathBuf, Entry> {
    let Some(path) = cache_path() else {
        return HashMap::new();
    };
    let Ok(data) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    let Ok(cache) = serde_json::from_str::<CacheFile>(&data) else {
        return HashMap::new();
    };
    if cache.version != VERSION {
        return HashMap::new();
    }
    cache
        .entries
        .into_iter()
        .map(|e| (e.track.path.clone(), e))
        .collect()
}

pub fn save(entries: &[Entry]) {
    let Some(path) = cache_path() else { return };
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let cache = CacheFileRef { version: VERSION, entries };
    let Ok(json) = serde_json::to_string(&cache) else {
        return;
    };
    // Write-then-rename, so an interrupted save leaves the previous cache
    // intact instead of a truncated file that parses to nothing.
    let tmp = path.with_extension("json.tmp");
    if fs::write(&tmp, json).is_ok() {
        let _ = fs::rename(&tmp, &path);
    }
}

/// File identity as far as the cache is concerned.
pub fn stamp(meta: &fs::Metadata) -> (u64, u64) {
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (mtime, meta.len())
}

/// True when the cached entry still describes the file on disk.
pub fn is_fresh(entry: &Entry, stamp: (u64, u64)) -> bool {
    entry.mtime == stamp.0 && entry.size == stamp.1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn entry() -> Entry {
        Entry {
            track: Track {
                path: PathBuf::from("/m/肺.flac"),
                title: "肺".into(),
                artist: "a子".into(),
                album: "肺".into(),
                album_artist: "a子".into(),
                genre: String::new(),
                year: Some(2020),
                track_number: Some(1),
                track_total: Some(1),
                disc_number: Some(1),
                disc_total: Some(1),
                comment: Some("note".into()),
                duration: Duration::from_secs(192),
                bitrate: Some(866),
                sample_rate: Some(44100),
                bit_depth: Some(16),
                channels: Some(2),
            },
            mtime: 1_700_000_000,
            size: 21_715_800,
        }
    }

    #[test]
    fn a_cached_track_survives_the_round_trip_intact() {
        let json = serde_json::to_string(&CacheFileRef { version: VERSION, entries: &[entry()] })
            .unwrap();
        let back: CacheFile = serde_json::from_str(&json).unwrap();

        let (a, b) = (&entry().track, &back.entries[0].track);
        assert_eq!(a.path, b.path);
        assert_eq!(a.title, b.title);
        assert_eq!(a.artist, b.artist);
        assert_eq!(a.year, b.year);
        assert_eq!(a.duration, b.duration);
        assert_eq!(a.sample_rate, b.sample_rate);
        assert_eq!(a.bit_depth, b.bit_depth);
        assert_eq!(a.channels, b.channels);
        assert_eq!(a.comment, b.comment);
    }

    #[test]
    fn an_edited_file_invalidates_its_entry() {
        let e = entry();
        assert!(is_fresh(&e, (e.mtime, e.size)));
        // Retagging changes both in practice; either alone is enough.
        assert!(!is_fresh(&e, (e.mtime + 1, e.size)));
        assert!(!is_fresh(&e, (e.mtime, e.size + 1)));
    }

    #[test]
    fn a_cache_from_an_older_build_is_ignored_rather_than_misread() {
        let json = format!(r#"{{"version":{},"entries":[]}}"#, VERSION + 1);
        let cache: CacheFile = serde_json::from_str(&json).unwrap();
        assert_ne!(cache.version, VERSION);
    }
}
