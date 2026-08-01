use std::path::{Path, PathBuf};
use std::time::SystemTime;

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;

use crate::app::state::InfoView;
use crate::app::App;
use crate::ui::theme::Theme;

// ── AlbumArtCache ────────────────────────────────────────────────────────

pub struct AlbumArtCache {
    track_dir: Option<PathBuf>,
    picker: Picker,
    protocol: Option<StatefulProtocol>,
}

impl AlbumArtCache {
    pub fn new(picker: Picker) -> Self {
        Self {
            track_dir: None,
            picker,
            protocol: None,
        }
    }

    fn needs_reload(&self, dir: Option<&Path>) -> bool {
        match (&self.track_dir, dir) {
            (Some(a), Some(b)) => a != b,
            (None, None) => false,
            _ => true,
        }
    }

    fn load(&mut self, dir: Option<&Path>) {
        self.track_dir = dir.map(|d| d.to_path_buf());
        self.protocol = None;

        let dir = match dir {
            Some(d) => d,
            None => return,
        };

        let cover_path = match find_cover_image(dir) {
            Some(p) => p,
            None => return,
        };

        let img = match image::open(&cover_path) {
            Ok(i) => i,
            Err(_) => return,
        };

        // StatefulProtocol handles resizing automatically per-frame
        self.protocol = Some(self.picker.new_resize_protocol(img));
    }
}

fn find_cover_image(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        // Check extension first (fast path)
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let l = ext.to_ascii_lowercase();
            if l == "jpg" || l == "jpeg" || l == "png" {
                return Some(path);
            }
        }
        // No extension or unknown ext — check magic bytes
        if is_image_by_magic(&path) {
            return Some(path);
        }
    }
    None
}

/// Check file header bytes to detect JPEG/PNG regardless of extension.
fn is_image_by_magic(path: &Path) -> bool {
    use std::fs::File;
    use std::io::Read;
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut buf = [0u8; 8];
    if f.read_exact(&mut buf).is_err() {
        return false;
    }
    // JPEG: FF D8 FF
    if buf[0] == 0xFF && buf[1] == 0xD8 && buf[2] == 0xFF {
        return true;
    }
    // PNG: 89 50 4E 47 0D 0A 1A 0A
    if buf == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        return true;
    }
    false
}

// ── Public render function ───────────────────────────────────────────────

pub fn render_info_pane(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    theme: &Theme,
    view: InfoView,
    art_cache: &mut AlbumArtCache,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_unfocused));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    match view {
        InfoView::Clock => render_clock(frame, inner, theme),
        InfoView::AlbumArt => render_album_art(frame, inner, app, art_cache),
    }
}

// ── Clock View ───────────────────────────────────────────────────────────

/// 5-line tall ASCII art digits using block characters
const DIGIT_PATTERNS: [&[&str]; 11] = [
    // 0
    &["████", "█  █", "█  █", "█  █", "████"],
    // 1
    &["  █ ", " ██ ", "  █ ", "  █ ", " ███"],
    // 2
    &["████", "   █", "████", "█   ", "████"],
    // 3
    &["████", "   █", "████", "   █", "████"],
    // 4
    &["█  █", "█  █", "████", "   █", "   █"],
    // 5
    &["████", "█   ", "████", "   █", "████"],
    // 6
    &["████", "█   ", "████", "█  █", "████"],
    // 7
    &["████", "   █", "  █ ", " █  ", " █  "],
    // 8
    &["████", "█  █", "████", "█  █", "████"],
    // 9
    &["████", "█  █", "████", "   █", "████"],
    // : (colon, index 10)
    &["    ", " ██ ", "    ", " ██ ", "    "],
];

fn render_clock(frame: &mut Frame, area: Rect, _theme: &Theme) {
    if area.width < 4 || area.height < 5 {
        return;
    }

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let (hours, minutes, _) = {
        let total_secs = now as i64;
        let mut tm = unsafe { std::mem::zeroed::<libc::tm>() };
        unsafe {
            libc::localtime_r(&total_secs as *const i64, &mut tm);
        }
        (tm.tm_hour as u32, tm.tm_min as u32, tm.tm_sec as u32)
    };

    // Build digit indices: HH:MM
    let digits = [
        (hours / 10) as usize,
        (hours % 10) as usize,
        10, // colon
        (minutes / 10) as usize,
        (minutes % 10) as usize,
    ];

    // Each digit is 4 chars wide + 1 space gap
    // Total: 5 * 4 + 4 gaps = 24 chars
    let total_width: u16 = 5 * 4 + 4;
    let x_offset = if area.width > total_width {
        (area.width - total_width) / 2
    } else {
        0
    };
    let y_offset = if area.height > 5 {
        (area.height - 5) / 2
    } else {
        0
    };

    // Gradient colors per digit: sky blue → cyan → mint → purple → pink
    let digit_colors: [Color; 5] = [
        Color::Rgb(100, 180, 255), // sky blue
        Color::Rgb(80, 220, 255),  // cyan
        Color::Rgb(200, 200, 200), // colon: white-gray
        Color::Rgb(200, 130, 255), // purple
        Color::Rgb(255, 130, 200), // pink
    ];

    for row in 0..5u16 {
        let y = area.y + y_offset + row;
        if y >= area.y + area.height {
            break;
        }
        let mut x = area.x + x_offset;
        for (i, &digit_idx) in digits.iter().enumerate() {
            let style = Style::default().fg(digit_colors[i % digit_colors.len()]);
            let pattern = DIGIT_PATTERNS[digit_idx][row as usize];
            for ch in pattern.chars() {
                if x >= area.x + area.width {
                    break;
                }
                let display_ch = if ch == '█' { '█' } else { ' ' };
                if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
                    cell.set_char(display_ch).set_style(style);
                }
                x += 1;
            }
            // Gap between digits
            if i < digits.len() - 1 {
                x += 1;
            }
        }
    }
}

// ── Album Art View ───────────────────────────────────────────────────────

fn render_album_art(frame: &mut Frame, area: Rect, app: &App, cache: &mut AlbumArtCache) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let track_dir = app.current_track().and_then(|t| t.path.parent().map(|p| p.to_path_buf()));

    if cache.needs_reload(track_dir.as_deref()) {
        cache.load(track_dir.as_deref());
    }

    match cache.protocol {
        Some(ref mut protocol) => {
            // Center the image area: most album art is square, so
            // compute a centered sub-rect matching the aspect ratio.
            let font = cache.picker.font_size();
            // Terminal cell aspect ratio: font_w / font_h
            // For a square image, we need w_cells * font_w == h_cells * font_h
            let (fw, fh) = (font.0.max(1) as u32, font.1.max(1) as u32);
            // Desired square in pixels: min(area.width * fw, area.height * fh)
            let px_w = area.width as u32 * fw;
            let px_h = area.height as u32 * fh;
            let side = px_w.min(px_h);
            let fit_w = (side / fw) as u16;
            let fit_h = (side / fh) as u16;
            let x_off = (area.width.saturating_sub(fit_w)) / 2;
            let y_off = (area.height.saturating_sub(fit_h)) / 2;
            let centered = Rect {
                x: area.x + x_off,
                y: area.y + y_off,
                width: fit_w.min(area.width),
                height: fit_h.min(area.height),
            };
            let widget = StatefulImage::default();
            frame.render_stateful_widget(widget, centered, protocol);
        }
        None => {
            // No album art placeholder
            let placeholder = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "\u{266A}",
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled("No Album Art", Style::default().fg(Color::DarkGray))),
            ];
            let para = Paragraph::new(placeholder).alignment(Alignment::Center);
            let v_offset = if area.height > 4 { (area.height - 4) / 2 } else { 0 };
            let centered = Rect {
                x: area.x,
                y: area.y + v_offset,
                width: area.width,
                height: area.height.saturating_sub(v_offset),
            };
            frame.render_widget(para, centered);
        }
    }
}

// ── Track Info View ──────────────────────────────────────────────────────

pub fn render_track_info(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let track = match app.current_track() {
        Some(t) => t,
        None => {
            let para = Paragraph::new("No track playing")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            frame.render_widget(para, area);
            return;
        }
    };

    let format_ext = track
        .path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown")
        .to_ascii_uppercase();

    let bitrate_str = track
        .bitrate
        .map(|b| format!("{} kbps", b))
        .unwrap_or_else(|| "N/A".to_string());

    let track_num_str = track
        .track_number
        .map(|n| n.to_string())
        .unwrap_or_else(|| "N/A".to_string());

    let duration_str = track.format_duration();


    // Each field has a unique label color and value style
    let fields: Vec<(&str, String, Color, Style)> = vec![
        ("Title", track.title.clone(),
            Color::Rgb(100, 180, 255),
            Style::default().fg(Color::Rgb(100, 220, 255)).add_modifier(Modifier::BOLD)),
        ("Artist", track.display_artist().to_string(),
            Color::Rgb(255, 180, 100),
            Style::default().fg(Color::Rgb(255, 210, 140))),
        ("Album", track.display_album().to_string(),
            Color::Rgb(200, 130, 255),
            Style::default().fg(Color::Rgb(220, 170, 255))),
        ("Album Artist",
            if track.album_artist.is_empty() { "N/A".to_string() } else { track.album_artist.clone() },
            Color::Rgb(200, 130, 255),
            Style::default().fg(theme.fg)),
        ("Genre",
            if track.genre.is_empty() { "N/A".to_string() } else { track.genre.clone() },
            Color::Rgb(255, 120, 150),
            Style::default().fg(Color::Rgb(255, 170, 190))),
        ("Track #", track_num_str,
            Color::Rgb(120, 220, 180),
            Style::default().fg(theme.fg)),
        ("Duration", duration_str,
            Color::Rgb(120, 220, 180),
            Style::default().fg(theme.fg)),
        ("Bitrate", bitrate_str,
            Color::Rgb(255, 220, 100),
            Style::default().fg(theme.fg)),
        ("Format", format_ext,
            Color::Rgb(255, 220, 100),
            Style::default().fg(theme.fg)),
    ];

    let lines: Vec<Line> = fields
        .iter()
        .map(|(label, value, label_color, val_style)| {
            Line::from(vec![
                Span::styled(format!("{:>13}: ", label), Style::default().fg(*label_color)),
                Span::styled(value.as_str(), *val_style),
            ])
        })
        .collect();

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}
