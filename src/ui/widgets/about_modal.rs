use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear};
use ratatui::Frame;

use crate::ui::theme::Theme;

// Neon colors
const NEON_PINK: Color = Color::Rgb(255, 50, 150);
const NEON_CYAN: Color = Color::Rgb(0, 255, 255);
const BG_DARK: Color = Color::Rgb(10, 5, 30);

// Logo gradient endpoints: hot pink -> cyan (smooth per-character interpolation)
const LOGO_START: (f32, f32, f32) = (255.0, 50.0, 150.0); // hot pink
const LOGO_END: (f32, f32, f32) = (0.0, 255.0, 255.0);    // cyan

// Dim background scatter colors
const DIM_PURPLE: Color = Color::Rgb(60, 20, 80);
const DIM_CYAN: Color = Color::Rgb(20, 60, 80);
const DIM_PINK: Color = Color::Rgb(80, 20, 50);

const NOTES: [char; 4] = ['♪', '♫', '♩', '◆'];
const DIM_COLORS: [Color; 3] = [DIM_PURPLE, DIM_CYAN, DIM_PINK];

// Waveform pattern for below-logo decoration
const WAVEFORM: &str = "▁▂▃▅▇█▇▅▃▂▁▂▃▅▇█▇▅▃▂▁";

// Unicode block-art logo — each row is 35 display columns
const LOGO: [&str; 5] = [
    " ██████  ██    ██ ██    ██ ██████  ",
    "██    ██ ███  ███ ███  ███ ██   ██ ",
    "██    ██ ██ ██ ██ ██ ██ ██ ██████  ",
    "██    ██ ██    ██ ██    ██ ██      ",
    " ██████  ██    ██ ██    ██ ██      ",
];
const LOGO_DISPLAY_W: u16 = 35;

/// Fill area with dark background + scattered music notes
fn render_background(buf: &mut Buffer, area: Rect) {
    let bg = Style::default().bg(BG_DARK).fg(BG_DARK);
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            buf.set_string(x, y, " ", bg);
        }
    }

    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let hash = (x as u32).wrapping_mul(7).wrapping_add((y as u32).wrapping_mul(13));
            if hash.is_multiple_of(37) {
                let note = NOTES[(hash as usize / 37) % NOTES.len()];
                let color = DIM_COLORS[(hash as usize / 37 + 1) % DIM_COLORS.len()];
                buf.set_string(x, y, note.to_string(), Style::default().fg(color).bg(BG_DARK));
            }
        }
    }
}

/// Render waveform with cyan->purple gradient, centered at given y
fn render_waveform(buf: &mut Buffer, area_x: u16, area_w: u16, y: u16, max_y: u16) {
    if y >= max_y {
        return;
    }
    let chars: Vec<char> = WAVEFORM.chars().collect();
    let total_w = chars.len() as u16;
    let start_x = area_x + area_w.saturating_sub(total_w) / 2;

    for (i, ch) in chars.iter().enumerate() {
        let x = start_x + i as u16;
        if x >= area_x && x < area_x + area_w {
            let frac = i as f32 / chars.len().max(1) as f32;
            let r = (frac * 160.0) as u8;
            let g = (255.0 - frac * 195.0) as u8;
            let b = (255.0 - frac * 15.0) as u8;
            buf.set_string(
                x, y, ch.to_string(),
                Style::default().fg(Color::Rgb(r, g, b)).bg(BG_DARK),
            );
        }
    }
}

/// Lerp a foreground color toward BG_DARK by opacity (0.0 = invisible, 1.0 = full color)
fn fade_color(target: Color, opacity: f32) -> Color {
    match target {
        Color::Rgb(r, g, b) => {
            let (br, bg, bb) = (10.0_f32, 5.0_f32, 30.0_f32);
            Color::Rgb(
                (br + (r as f32 - br) * opacity) as u8,
                (bg + (g as f32 - bg) * opacity) as u8,
                (bb + (b as f32 - bb) * opacity) as u8,
            )
        }
        other => other,
    }
}

/// Helper: center a string of `display_w` columns within `area_w`, returning x offset from area.x
fn center_x(area_x: u16, area_w: u16, display_w: u16) -> u16 {
    area_x + area_w.saturating_sub(display_w) / 2
}

pub fn render_about_modal(frame: &mut Frame, area: Rect, _theme: &Theme) {
    let modal = centered_rect(50, 60, area);
    frame.render_widget(Clear, modal);

    // Background with scattered music notes on entire modal
    render_background(frame.buffer_mut(), modal);

    // Modal border
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(NEON_PINK).bg(BG_DARK))
        .title(" ♪ OMMP ♪ ")
        .title_style(
            Style::default()
                .fg(NEON_PINK)
                .bg(BG_DARK)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    // Re-render background inside inner area (border rendering overwrites)
    render_background(frame.buffer_mut(), inner);

    // === Calculate total content height for vertical centering ===
    // Logo(5) + waveform(1) + 1 + subtitle(1) + tagline(1) + 1 + divider(1) + 1 + info(2) + 1 + links(2) + 1 + divider(1) + 1 + footer(1) = 21
    let content_h: u16 = 21;
    let top_pad = inner.height.saturating_sub(content_h) / 2;
    let mut cur_y = inner.y + top_pad;

    let buf = frame.buffer_mut();

    // --- Logo with smooth horizontal gradient (hot pink -> cyan, per-character RGB lerp) ---
    for (row_idx, row) in LOGO.iter().enumerate() {
        if cur_y < inner.y + inner.height {
            let logo_x = center_x(inner.x, inner.width, LOGO_DISPLAY_W);
            for (col, ch) in row.chars().enumerate() {
                let t = (col as f32 + row_idx as f32 * 3.0)
                    / (LOGO_DISPLAY_W as f32 + 4.0 * 3.0);
                let t = t.clamp(0.0, 1.0);
                let r = (LOGO_START.0 + (LOGO_END.0 - LOGO_START.0) * t) as u8;
                let g = (LOGO_START.1 + (LOGO_END.1 - LOGO_START.1) * t) as u8;
                let b = (LOGO_START.2 + (LOGO_END.2 - LOGO_START.2) * t) as u8;
                let style = Style::default()
                    .fg(Color::Rgb(r, g, b))
                    .bg(BG_DARK)
                    .add_modifier(Modifier::BOLD);
                buf.set_string(logo_x + col as u16, cur_y, ch.to_string(), style);
            }
        }
        cur_y += 1;
    }

    // --- Waveform below logo ---
    render_waveform(buf, inner.x, inner.width, cur_y, inner.y + inner.height);
    cur_y += 2;

    // --- Subtitle ---
    let subtitle = "Oh My Music Player";
    let sub_w = subtitle.len() as u16;
    if cur_y < inner.y + inner.height {
        let x = center_x(inner.x, inner.width, sub_w);
        buf.set_string(
            x, cur_y, subtitle,
            Style::default()
                .fg(NEON_CYAN)
                .bg(BG_DARK)
                .add_modifier(Modifier::BOLD | Modifier::ITALIC),
        );
    }
    cur_y += 1;

    // --- Tagline ---
    let tagline = "Terminal music, your way";
    let tag_w = tagline.len() as u16;
    if cur_y < inner.y + inner.height {
        let x = center_x(inner.x, inner.width, tag_w);
        buf.set_string(x, cur_y, tagline, Style::default().fg(NEON_CYAN).bg(BG_DARK));
    }
    cur_y += 2;

    // --- Divider ---
    let div_w = inner.width.saturating_sub(6) as usize;
    let div_style = Style::default().fg(DIM_PURPLE).bg(BG_DARK);
    if cur_y < inner.y + inner.height {
        buf.set_string(inner.x + 3, cur_y, "─".repeat(div_w), div_style);
    }
    cur_y += 2;

    // --- Info rows ---
    let label_style = Style::default()
        .fg(NEON_PINK)
        .bg(BG_DARK)
        .add_modifier(Modifier::BOLD);
    let value_style = Style::default().fg(NEON_CYAN).bg(BG_DARK);

    for (label, value) in &[("Version", "0.1.0"), ("License", "MIT")] {
        if cur_y < inner.y + inner.height {
            let lbl = format!("  {:12}", label);
            let val = *value;
            let total_w = (lbl.len() + val.len()) as u16;
            let x = center_x(inner.x, inner.width, total_w);
            buf.set_string(x, cur_y, &lbl, label_style);
            buf.set_string(x + lbl.len() as u16, cur_y, val, value_style);
        }
        cur_y += 1;
    }
    cur_y += 1;

    // --- Links ---
    let link_style = Style::default()
        .fg(Color::Rgb(100, 180, 255))
        .bg(BG_DARK)
        .add_modifier(Modifier::UNDERLINED);
    let icon_style = Style::default()
        .fg(NEON_PINK)
        .bg(BG_DARK)
        .add_modifier(Modifier::BOLD);

    for (icon, url) in &[
        ("\u{F09B} ", "https://github.com/sqzer-x/ommp"),
        ("\u{F004} ", "https://github.com/sponsors/sqzer-x"),
    ] {
        if cur_y < inner.y + inner.height {
            let icon_w = icon.chars().count() as u16;
            let url_w = url.len() as u16;
            let total_w = icon_w + url_w;
            let x = center_x(inner.x, inner.width, total_w);
            buf.set_string(x, cur_y, *icon, icon_style);
            buf.set_string(x + icon_w, cur_y, *url, link_style);
        }
        cur_y += 1;
    }
    cur_y += 1;

    // --- Divider ---
    if cur_y < inner.y + inner.height {
        buf.set_string(inner.x + 3, cur_y, "─".repeat(div_w), div_style);
    }
    cur_y += 2;

    // --- Footer ---
    let footer = "g: Open GitHub  s: Open Sponsor  Esc: Close";
    let fw = footer.len() as u16;
    if cur_y < inner.y + inner.height {
        let x = center_x(inner.x, inner.width, fw);
        buf.set_string(
            x, cur_y, footer,
            Style::default().fg(Color::Rgb(120, 120, 140)).bg(BG_DARK),
        );
    }
}

/// Render a full-screen splash screen with fade-in/fade-out.
/// `opacity`: 0.0 = fully transparent (BG_DARK only), 1.0 = full brightness.
pub fn render_splash_screen(frame: &mut Frame, area: Rect, _theme: &Theme, opacity: f32) {
    let opacity = opacity.clamp(0.0, 1.0);
    let buf = frame.buffer_mut();

    // Fill entire screen with dark background
    let bg = Style::default().bg(BG_DARK).fg(BG_DARK);
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            buf.set_string(x, y, " ", bg);
        }
    }

    // Scattered music notes (faded)
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let hash = (x as u32).wrapping_mul(7).wrapping_add((y as u32).wrapping_mul(13));
            if hash.is_multiple_of(37) {
                let note = NOTES[(hash as usize / 37) % NOTES.len()];
                let color = DIM_COLORS[(hash as usize / 37 + 1) % DIM_COLORS.len()];
                buf.set_string(
                    x, y, note.to_string(),
                    Style::default().fg(fade_color(color, opacity)).bg(BG_DARK),
                );
            }
        }
    }

    // Content height: logo(5) + waveform(1) + gap(1) + subtitle(1) + tagline(1) = 9
    let content_h: u16 = 9;
    let top_pad = area.height.saturating_sub(content_h) / 2;
    let mut cur_y = area.y + top_pad;

    // --- Logo with smooth horizontal gradient (faded) ---
    for (row_idx, row) in LOGO.iter().enumerate() {
        if cur_y < area.y + area.height {
            let logo_x = center_x(area.x, area.width, LOGO_DISPLAY_W);
            for (col, ch) in row.chars().enumerate() {
                let t = (col as f32 + row_idx as f32 * 3.0)
                    / (LOGO_DISPLAY_W as f32 + 4.0 * 3.0);
                let t = t.clamp(0.0, 1.0);
                let r = (LOGO_START.0 + (LOGO_END.0 - LOGO_START.0) * t) as u8;
                let g = (LOGO_START.1 + (LOGO_END.1 - LOGO_START.1) * t) as u8;
                let b = (LOGO_START.2 + (LOGO_END.2 - LOGO_START.2) * t) as u8;
                let style = Style::default()
                    .fg(fade_color(Color::Rgb(r, g, b), opacity))
                    .bg(BG_DARK)
                    .add_modifier(Modifier::BOLD);
                buf.set_string(logo_x + col as u16, cur_y, ch.to_string(), style);
            }
        }
        cur_y += 1;
    }

    // --- Waveform below logo (faded) ---
    if cur_y < area.y + area.height {
        let chars: Vec<char> = WAVEFORM.chars().collect();
        let total_w = chars.len() as u16;
        let start_x = area.x + area.width.saturating_sub(total_w) / 2;
        for (i, ch) in chars.iter().enumerate() {
            let x = start_x + i as u16;
            if x >= area.x && x < area.x + area.width {
                let frac = i as f32 / chars.len().max(1) as f32;
                let r = (frac * 160.0) as u8;
                let g = (255.0 - frac * 195.0) as u8;
                let b = (255.0 - frac * 15.0) as u8;
                buf.set_string(
                    x, cur_y, ch.to_string(),
                    Style::default().fg(fade_color(Color::Rgb(r, g, b), opacity)).bg(BG_DARK),
                );
            }
        }
    }
    cur_y += 2;

    // --- Subtitle (faded) ---
    let subtitle = "Oh My Music Player";
    let sub_w = subtitle.len() as u16;
    if cur_y < area.y + area.height {
        let x = center_x(area.x, area.width, sub_w);
        buf.set_string(
            x, cur_y, subtitle,
            Style::default()
                .fg(fade_color(NEON_CYAN, opacity))
                .bg(BG_DARK)
                .add_modifier(Modifier::BOLD | Modifier::ITALIC),
        );
    }
    cur_y += 1;

    // --- Tagline (faded) ---
    let tagline = "Terminal music, your way";
    let tag_w = tagline.len() as u16;
    if cur_y < area.y + area.height {
        let x = center_x(area.x, area.width, tag_w);
        buf.set_string(
            x, cur_y, tagline,
            Style::default().fg(fade_color(NEON_CYAN, opacity)).bg(BG_DARK),
        );
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
