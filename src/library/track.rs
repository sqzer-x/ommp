use lofty::file::AudioFile;
use lofty::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genre: String,
    pub track_number: Option<u32>,
    pub duration: Duration,
    pub bitrate: Option<u32>,
    #[allow(dead_code)]
    pub lyrics: Option<String>,
}

impl Track {
    pub fn from_path(path: &Path) -> Option<Self> {
        let tagged_file = lofty::read_from_path(path).ok()?;

        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag());

        let properties = tagged_file.properties();
        let duration = properties.duration();
        let bitrate = properties.audio_bitrate();

        let (title, artist, album, album_artist, genre, track_number, lyrics) =
            if let Some(tag) = tag {
                let title_str: String = tag.title().map(|s| s.to_string()).unwrap_or_default();
                let artist_str: String = tag.artist().map(|s| s.to_string()).unwrap_or_default();
                let album_str: String = tag.album().map(|s| s.to_string()).unwrap_or_default();
                let aa_str: String = tag
                    .get_string(&ItemKey::AlbumArtist)
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let genre_str: String = tag.genre().map(|s| s.to_string()).unwrap_or_default();
                let track_num = tag.track();
                let lyrics_str: Option<String> = tag
                    .get_string(&ItemKey::Lyrics)
                    .map(|s| s.to_string());
                (title_str, artist_str, album_str, aa_str, genre_str, track_num, lyrics_str)
            } else {
                (String::new(), String::new(), String::new(), String::new(), String::new(), None, None)
            };

        let title = if title.is_empty() {
            path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            title
        };

        Some(Self {
            path: path.to_path_buf(),
            title,
            artist,
            album,
            album_artist,
            genre,
            track_number,
            duration,
            bitrate,
            lyrics,
        })
    }

    pub fn display_artist(&self) -> &str {
        if self.artist.is_empty() {
            "Unknown Artist"
        } else {
            &self.artist
        }
    }

    pub fn display_album(&self) -> &str {
        if self.album.is_empty() {
            "Unknown Album"
        } else {
            &self.album
        }
    }

    pub fn format_duration(&self) -> String {
        let secs = self.duration.as_secs();
        let mins = secs / 60;
        let secs = secs % 60;
        format!("{}:{:02}", mins, secs)
    }
}
