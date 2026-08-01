pub mod scanner;
pub mod track;
pub mod watcher;

use std::collections::BTreeSet;
use std::path::Path;
use track::Track;

#[derive(Debug)]
pub struct Library {
    pub tracks: Vec<Track>,
}

impl Library {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
        }
    }

    pub fn scan(path: &Path) -> Self {
        let tracks = scanner::scan_directory(path);
        Self { tracks }
    }

    pub fn get_artists(&self) -> Vec<String> {
        let mut set = BTreeSet::new();
        let mut has_unknown = false;
        for t in &self.tracks {
            if t.artist.is_empty() {
                has_unknown = true;
            } else {
                set.insert(t.artist.clone());
            }
        }
        let mut result: Vec<String> = set.into_iter().collect();
        if has_unknown {
            result.push("Unknown Artist".to_string());
        }
        result
    }

    #[allow(dead_code)]
    pub fn get_album_artists(&self) -> Vec<String> {
        let mut set = BTreeSet::new();
        for t in &self.tracks {
            if !t.album_artist.is_empty() {
                set.insert(t.album_artist.clone());
            }
        }
        set.into_iter().collect()
    }

    pub fn get_genres(&self) -> Vec<String> {
        let mut set = BTreeSet::new();
        for t in &self.tracks {
            if !t.genre.is_empty() {
                set.insert(t.genre.clone());
            }
        }
        set.into_iter().collect()
    }

    pub fn get_albums(&self) -> Vec<(String, String)> {
        let mut set = BTreeSet::new();
        for t in &self.tracks {
            if !t.album.is_empty() {
                let artist = if t.album_artist.is_empty() {
                    t.artist.clone()
                } else {
                    t.album_artist.clone()
                };
                set.insert((t.album.clone(), artist));
            }
        }
        set.into_iter().collect()
    }

    pub fn get_tracks_by_artist(&self, artist: &str) -> Vec<usize> {
        self.tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                if artist == "Unknown Artist" {
                    t.artist.is_empty()
                } else {
                    t.artist == artist
                }
            })
            .map(|(i, _)| i)
            .collect()
    }

    #[allow(dead_code)]
    pub fn get_tracks_by_album_artist(&self, album_artist: &str) -> Vec<usize> {
        self.tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.album_artist == album_artist)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn get_tracks_by_album(&self, album: &str) -> Vec<usize> {
        self.tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.album == album)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn get_tracks_by_genre(&self, genre: &str) -> Vec<usize> {
        self.tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.genre == genre)
            .map(|(i, _)| i)
            .collect()
    }

    #[allow(dead_code)]
    pub fn get_albums_by_album_artist(&self, album_artist: &str) -> Vec<String> {
        let mut set = BTreeSet::new();
        for t in &self.tracks {
            if t.album_artist == album_artist && !t.album.is_empty() {
                set.insert(t.album.clone());
            }
        }
        set.into_iter().collect()
    }

    pub fn get_formats(&self) -> Vec<String> {
        let mut set = BTreeSet::new();
        for t in &self.tracks {
            let ext = t.path.extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if !ext.is_empty() {
                set.insert(ext);
            }
        }
        set.into_iter().collect()
    }

    pub fn get_tracks_by_format(&self, format: &str) -> Vec<usize> {
        self.tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                t.path.extension()
                    .map(|e| e.to_string_lossy().to_lowercase() == format)
                    .unwrap_or(false)
            })
            .map(|(i, _)| i)
            .collect()
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

    pub fn path_to_index(&self, path: &Path) -> Option<usize> {
        self.tracks.iter().position(|t| t.path == path)
    }

    pub fn search(&self, query: &str) -> Vec<usize> {
        if query.is_empty() {
            return Vec::new();
        }

        // Extension filter: *.flac, *.mp3, etc.
        if let Some(ext) = query.strip_prefix("*.") {
            let ext = ext.to_lowercase();
            return self.tracks.iter().enumerate()
                .filter(|(_, t)| {
                    t.path.extension()
                        .map(|e| e.to_string_lossy().to_lowercase() == ext)
                        .unwrap_or(false)
                })
                .map(|(i, _)| i)
                .collect();
        }

        // Field-specific filter: artist:, album:, genre:, title:
        if let Some((prefix, value)) = query.split_once(':') {
            let field = prefix.trim().to_lowercase();
            let v = value.trim().to_lowercase();
            if !v.is_empty() {
                match field.as_str() {
                    "artist" => {
                        return self.tracks.iter().enumerate()
                            .filter(|(_, t)| t.artist.to_lowercase().contains(&v))
                            .map(|(i, _)| i).collect();
                    }
                    "album" => {
                        return self.tracks.iter().enumerate()
                            .filter(|(_, t)| t.album.to_lowercase().contains(&v))
                            .map(|(i, _)| i).collect();
                    }
                    "genre" => {
                        return self.tracks.iter().enumerate()
                            .filter(|(_, t)| t.genre.to_lowercase().contains(&v))
                            .map(|(i, _)| i).collect();
                    }
                    "title" => {
                        return self.tracks.iter().enumerate()
                            .filter(|(_, t)| t.title.to_lowercase().contains(&v))
                            .map(|(i, _)| i).collect();
                    }
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
                t.title.to_lowercase().contains(&q)
                    || t.artist.to_lowercase().contains(&q)
                    || t.album.to_lowercase().contains(&q)
                    || t.genre.to_lowercase().contains(&q)
                    || t.path.file_name()
                        .map(|f| f.to_string_lossy().to_lowercase().contains(&q))
                        .unwrap_or(false)
            })
            .map(|(i, _)| i)
            .collect()
    }
}
