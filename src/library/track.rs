use lofty::config::ParseOptions;
use lofty::file::AudioFile;
use lofty::prelude::*;
use lofty::probe::Probe;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Serializable so the scan cache can store parsed tags verbatim; every field
/// here came out of the file, so restoring one is equivalent to re-reading it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genre: String,
    pub year: Option<u32>,
    pub track_number: Option<u32>,
    pub track_total: Option<u32>,
    pub disc_number: Option<u32>,
    pub disc_total: Option<u32>,
    pub comment: Option<String>,
    pub duration: Duration,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub channels: Option<u8>,
}

impl Track {
    pub fn from_path(path: &Path) -> Option<Self> {
        // Skip cover art: scanning never keeps it, and the album art view reads
        // the picture straight from the file when it actually needs one. lofty
        // otherwise decodes and allocates every embedded image — around a
        // megabyte per track — only to have it dropped.
        let tagged_file = Probe::open(path)
            .ok()?
            .options(ParseOptions::new().read_cover_art(false))
            .read()
            .ok()?;

        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag());

        let properties = tagged_file.properties();

        let title = tag
            .and_then(|t| t.title())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
            });

        Some(Self {
            path: path.to_path_buf(),
            title,
            artist: tag.and_then(|t| t.artist()).map(|s| s.to_string()).unwrap_or_default(),
            album: tag.and_then(|t| t.album()).map(|s| s.to_string()).unwrap_or_default(),
            album_artist: tag
                .and_then(|t| t.get_string(&ItemKey::AlbumArtist))
                .map(|s| s.to_string())
                .unwrap_or_default(),
            genre: tag.and_then(|t| t.genre()).map(|s| s.to_string()).unwrap_or_default(),
            year: tag.and_then(|t| t.year()),
            track_number: tag.and_then(|t| t.track()),
            track_total: tag.and_then(|t| t.track_total()),
            disc_number: tag.and_then(|t| t.disk()),
            disc_total: tag.and_then(|t| t.disk_total()),
            comment: tag
                .and_then(|t| t.comment())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty()),
            duration: properties.duration(),
            // FLAC and other lossless formats often only carry the overall rate.
            bitrate: properties.audio_bitrate().or_else(|| properties.overall_bitrate()),
            sample_rate: properties.sample_rate(),
            bit_depth: properties.bit_depth(),
            channels: properties.channels(),
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

    /// "1/12", or just "1" when the total is unknown.
    pub fn format_index(n: Option<u32>, total: Option<u32>) -> String {
        match (n, total) {
            (Some(n), Some(total)) => format!("{}/{}", n, total),
            (Some(n), None) => n.to_string(),
            _ => "N/A".to_string(),
        }
    }

    /// "44.1 kHz · 16-bit · Stereo" from whichever parts the file reports.
    pub fn format_quality(&self) -> String {
        let mut parts = Vec::new();
        if let Some(hz) = self.sample_rate {
            parts.push(format!("{:.1} kHz", hz as f64 / 1000.0));
        }
        if let Some(bits) = self.bit_depth {
            parts.push(format!("{}-bit", bits));
        }
        match self.channels {
            Some(1) => parts.push("Mono".to_string()),
            Some(2) => parts.push("Stereo".to_string()),
            Some(n) => parts.push(format!("{} ch", n)),
            None => {}
        }
        if parts.is_empty() {
            "N/A".to_string()
        } else {
            parts.join(" \u{00B7} ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(sample_rate: Option<u32>, bit_depth: Option<u8>, channels: Option<u8>) -> Track {
        Track {
            path: PathBuf::new(),
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            album_artist: String::new(),
            genre: String::new(),
            year: None,
            track_number: None,
            track_total: None,
            disc_number: None,
            disc_total: None,
            comment: None,
            duration: Duration::ZERO,
            bitrate: None,
            sample_rate,
            bit_depth,
            channels,
        }
    }

    #[test]
    fn index_uses_total_only_when_present() {
        assert_eq!(Track::format_index(Some(1), Some(12)), "1/12");
        assert_eq!(Track::format_index(Some(1), None), "1");
        assert_eq!(Track::format_index(None, Some(12)), "N/A");
        assert_eq!(Track::format_index(None, None), "N/A");
    }

    #[test]
    fn quality_skips_parts_the_file_does_not_report() {
        assert_eq!(
            track(Some(44100), Some(16), Some(2)).format_quality(),
            "44.1 kHz \u{00B7} 16-bit \u{00B7} Stereo"
        );
        assert_eq!(track(Some(48000), None, Some(1)).format_quality(), "48.0 kHz \u{00B7} Mono");
        assert_eq!(track(None, None, Some(6)).format_quality(), "6 ch");
        assert_eq!(track(None, None, None).format_quality(), "N/A");
    }
}
