pub mod scanner;
pub mod track;
pub mod watcher;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use track::Track;

/// Tracks with no artist tag are grouped under this name.
pub const UNKNOWN_ARTIST: &str = "Unknown Artist";

/// Views derived from `tracks`, built once when the library is constructed.
///
/// Every one of these used to be recomputed on each access — `get_albums` alone
/// walked all tracks and cloned two strings each — and the render loop calls
/// several of them per frame at up to 20 frames a second.
#[derive(Debug, Default)]
struct Index {
    artists: Vec<String>,
    albums: Vec<(String, String)>,
    genres: Vec<String>,
    /// Format extension with how many tracks use it.
    formats: Vec<(String, usize)>,
    /// Basenames of every directory containing a track.
    directories: Vec<String>,
    by_artist: HashMap<String, Vec<usize>>,
    by_album: HashMap<(String, String), Vec<usize>>,
    by_genre: HashMap<String, Vec<usize>>,
    by_format: HashMap<String, Vec<usize>>,
    by_path: HashMap<PathBuf, usize>,
}

#[derive(Debug)]
pub struct Library {
    pub tracks: Vec<Track>,
    index: Index,
}

impl Library {
    pub fn new() -> Self {
        Self { tracks: Vec::new(), index: Index::default() }
    }

    pub fn from_tracks(tracks: Vec<Track>) -> Self {
        let index = Index::build(&tracks);
        Self { tracks, index }
    }

    pub fn scan(path: &Path) -> Self {
        Self::from_tracks(scanner::scan_directory(path))
    }

    // ── Derived views: O(1), the work happened at construction ───────────

    pub fn artists(&self) -> &[String] {
        &self.index.artists
    }

    pub fn albums(&self) -> &[(String, String)] {
        &self.index.albums
    }

    pub fn genres(&self) -> &[String] {
        &self.index.genres
    }

    /// Format extensions paired with their track counts.
    pub fn formats(&self) -> &[(String, usize)] {
        &self.index.formats
    }

    pub fn directories(&self) -> &[String] {
        &self.index.directories
    }

    // ── Lookups: one hash, then a slice ─────────────────────────────────

    pub fn tracks_by_artist(&self, artist: &str) -> &[usize] {
        Self::slice(self.index.by_artist.get(artist))
    }

    /// Tracks on `album` by `artist`. Matching on the title alone merged two
    /// different records that happen to share a name, since `albums()` pairs
    /// each album with its artist.
    pub fn tracks_by_album(&self, album: &str, artist: &str) -> &[usize] {
        Self::slice(self.index.by_album.get(&(album.to_string(), artist.to_string())))
    }

    pub fn tracks_by_genre(&self, genre: &str) -> &[usize] {
        Self::slice(self.index.by_genre.get(genre))
    }

    pub fn tracks_by_format(&self, format: &str) -> &[usize] {
        Self::slice(self.index.by_format.get(format))
    }

    fn slice(v: Option<&Vec<usize>>) -> &[usize] {
        v.map(Vec::as_slice).unwrap_or(&[])
    }

    /// Was a linear scan over every path, so restoring a 1,000-entry playlist
    /// against a 10,000-track library cost 10 million path comparisons.
    pub fn path_to_index(&self, path: &Path) -> Option<usize> {
        self.index.by_path.get(path).copied()
    }

    /// The artist an album is filed under: the album artist when tagged,
    /// otherwise the track artist.
    fn album_artist_of(t: &Track) -> &str {
        if t.album_artist.is_empty() {
            &t.artist
        } else {
            &t.album_artist
        }
    }

    fn extension_of(t: &Track) -> String {
        t.path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    }

    pub fn get_directory_entries(&self, dir: &Path) -> (Vec<String>, Vec<usize>) {
        let mut subdirs = BTreeSet::new();
        let mut tracks = Vec::new();

        for (i, t) in self.tracks.iter().enumerate() {
            if let Some(parent) = t.path.parent() {
                if parent == dir {
                    tracks.push(i);
                } else if let Ok(rel) = parent.strip_prefix(dir) {
                    if let Some(first) = rel.components().next() {
                        subdirs.insert(first.as_os_str().to_string_lossy().to_string());
                    }
                }
            }
        }

        (subdirs.into_iter().collect(), tracks)
    }

    pub fn search(&self, query: &str) -> Vec<usize> {
        if query.is_empty() {
            return Vec::new();
        }

        // Extension filter: *.flac, *.mp3, etc. — now an index lookup.
        if let Some(ext) = query.strip_prefix("*.") {
            return self.tracks_by_format(&ext.to_lowercase()).to_vec();
        }

        // Field-specific filter: artist:, album:, genre:, title:
        if let Some((prefix, value)) = query.split_once(':') {
            let field = prefix.trim().to_lowercase();
            let v = value.trim().to_lowercase();
            if !v.is_empty() {
                let pick = |f: fn(&Track) -> &str| {
                    self.tracks
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| contains_ci(f(t), &v))
                        .map(|(i, _)| i)
                        .collect::<Vec<_>>()
                };
                match field.as_str() {
                    "artist" => return pick(|t| &t.artist),
                    "album" => return pick(|t| &t.album),
                    "genre" => return pick(|t| &t.genre),
                    "title" => return pick(|t| &t.title),
                    _ => {} // unknown prefix, fall through to general search
                }
            }
        }

        // General search: title, artist, album, genre, filename
        let q = query.to_lowercase();
        self.tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                contains_ci(&t.title, &q)
                    || contains_ci(&t.artist, &q)
                    || contains_ci(&t.album, &q)
                    || contains_ci(&t.genre, &q)
                    || t.path
                        .file_name()
                        .is_some_and(|f| contains_ci(&f.to_string_lossy(), &q))
            })
            .map(|(i, _)| i)
            .collect()
    }
}

/// Case-insensitive substring test that does not allocate for ASCII text.
/// Search ran `to_lowercase()` on five fields of every track for every
/// keystroke, which is five heap allocations per track per character typed.
fn contains_ci(haystack: &str, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    if haystack.is_ascii() {
        let hay = haystack.as_bytes();
        let ned = needle_lower.as_bytes();
        return ned.len() <= hay.len()
            && hay.windows(ned.len()).any(|w| w.eq_ignore_ascii_case(ned));
    }
    haystack.to_lowercase().contains(needle_lower)
}

impl Index {
    fn build(tracks: &[Track]) -> Self {
        let mut by_artist: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_album: HashMap<(String, String), Vec<usize>> = HashMap::new();
        let mut by_genre: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_format: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_path: HashMap<PathBuf, usize> = HashMap::with_capacity(tracks.len());
        let mut directories: BTreeSet<String> = BTreeSet::new();

        for (i, t) in tracks.iter().enumerate() {
            let artist = if t.artist.is_empty() { UNKNOWN_ARTIST } else { &t.artist };
            by_artist.entry(artist.to_string()).or_default().push(i);

            if !t.album.is_empty() {
                let key = (t.album.clone(), Library::album_artist_of(t).to_string());
                by_album.entry(key).or_default().push(i);
            }
            if !t.genre.is_empty() {
                by_genre.entry(t.genre.clone()).or_default().push(i);
            }
            let ext = Library::extension_of(t);
            if !ext.is_empty() {
                by_format.entry(ext).or_default().push(i);
            }
            // First path wins, matching the old `position()` behaviour when a
            // symlink makes the same file appear under two paths.
            by_path.entry(t.path.clone()).or_insert(i);

            if let Some(name) = t
                .path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
            {
                directories.insert(name.to_string());
            }
        }

        // "Unknown Artist" sorts last, as it did when it was appended after the
        // sorted set.
        let mut artists: Vec<String> = by_artist.keys().cloned().collect();
        artists.sort_by(|a, b| match (a == UNKNOWN_ARTIST, b == UNKNOWN_ARTIST) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => a.cmp(b),
        });

        let albums: Vec<(String, String)> =
            by_album.keys().cloned().collect::<BTreeSet<_>>().into_iter().collect();
        let genres: Vec<String> =
            by_genre.keys().cloned().collect::<BTreeSet<_>>().into_iter().collect();
        let formats: Vec<(String, usize)> = by_format
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .collect();

        Self {
            artists,
            albums,
            genres,
            formats,
            directories: directories.into_iter().collect(),
            by_artist,
            by_album,
            by_genre,
            by_format,
            by_path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn t(artist: &str, album_artist: &str, album: &str, genre: &str, path: &str) -> Track {
        Track {
            path: PathBuf::from(path),
            title: path.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            album_artist: album_artist.to_string(),
            genre: genre.to_string(),
            year: None,
            track_number: None,
            track_total: None,
            disc_number: None,
            disc_total: None,
            comment: None,
            duration: Duration::ZERO,
            bitrate: None,
            sample_rate: None,
            bit_depth: None,
            channels: None,
        }
    }

    fn library() -> Library {
        Library::from_tracks(vec![
            t("b", "", "Hits", "rock", "/m/b/1.flac"),
            t("a", "", "Hits", "pop", "/m/a/2.mp3"),
            t("", "", "Loose", "", "/m/c/3.flac"),
            t("a", "", "Hits", "pop", "/m/a/4.flac"),
        ])
    }

    #[test]
    fn artists_are_sorted_with_unknown_last() {
        assert_eq!(library().artists(), ["a", "b", UNKNOWN_ARTIST]);
    }

    #[test]
    fn albums_keep_their_artist_so_same_named_records_stay_separate() {
        let lib = library();
        // "Hits" exists under both a and b and must not be merged.
        assert_eq!(
            lib.albums(),
            [
                ("Hits".into(), "a".into()),
                ("Hits".into(), "b".into()),
                ("Loose".into(), "".into()),
            ]
        );
        assert_eq!(lib.tracks_by_album("Hits", "a"), [1, 3]);
        assert_eq!(lib.tracks_by_album("Hits", "b"), [0]);
    }

    #[test]
    fn untagged_tracks_group_under_unknown_artist() {
        assert_eq!(library().tracks_by_artist(UNKNOWN_ARTIST), [2]);
        assert_eq!(library().tracks_by_artist("a"), [1, 3]);
        assert!(library().tracks_by_artist("nobody").is_empty());
    }

    #[test]
    fn formats_carry_their_counts_and_index_their_tracks() {
        let lib = library();
        assert_eq!(lib.formats(), [("flac".into(), 3usize), ("mp3".into(), 1)]);
        assert_eq!(lib.tracks_by_format("flac"), [0, 2, 3]);
    }

    #[test]
    fn lookups_resolve_by_path_and_directory() {
        let lib = library();
        assert_eq!(lib.path_to_index(Path::new("/m/a/4.flac")), Some(3));
        assert_eq!(lib.path_to_index(Path::new("/m/nope.flac")), None);
        assert_eq!(lib.directories(), ["a", "b", "c"]);
    }

    #[test]
    fn extension_search_uses_the_index_and_matches_the_scan() {
        assert_eq!(library().search("*.flac"), vec![0, 2, 3]);
    }

    #[test]
    fn case_insensitive_match_agrees_with_the_allocating_version() {
        for (hay, needle) in [
            ("Ghost In A Flower", "flower"),
            ("Ghost In A Flower", "GHOST"),
            ("肺", "肺"),
            ("ヨルシカ Elma", "elma"),
            ("short", "much longer needle"),
            ("anything", ""),
        ] {
            assert_eq!(
                contains_ci(hay, &needle.to_lowercase()),
                hay.to_lowercase().contains(&needle.to_lowercase()),
                "{hay:?} contains {needle:?}"
            );
        }
    }
}
