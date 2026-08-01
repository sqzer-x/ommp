use ratatui::layout::{Constraint, Direction, Layout, Rect, Alignment};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::app::state::{PlayState, SyncState};
use crate::ui::theme::Theme;

pub fn render_status_bar(frame: &mut Frame, area: Rect, app: &App, theme: &Theme, resize_mode: bool) {
    let block = if resize_mode {
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" [RESIZE] ")
            .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    } else if app.sync_state == SyncState::Scanning {
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(255, 200, 80)))
            .title(" [SYNCING] ")
            .title_style(Style::default().fg(Color::Rgb(255, 200, 80)).add_modifier(Modifier::BOLD))
    } else {
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_unfocused))
    };

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(inner);

    // Left: Play state + time
    let state_icon = match app.playback.state {
        PlayState::Playing => "\u{F04B}",  // nf-fa-play
        PlayState::Paused => "\u{F04C}",   // nf-fa-pause
        PlayState::Stopped => "\u{F04D}",  // nf-fa-stop
    };

    let pos = format_time(app.playback.position_secs);
    let dur = format_time(app.playback.duration_secs);

    let bitrate = app
        .current_track()
        .and_then(|t| t.bitrate)
        .map(|b| format!(" ({}kbps)", b))
        .unwrap_or_default();

    let state_color = match app.playback.state {
        PlayState::Playing => Color::Rgb(80, 255, 120),   // bright green
        PlayState::Paused => Color::Rgb(255, 200, 80),    // amber
        PlayState::Stopped => Color::Rgb(255, 100, 100),  // soft red
    };

    let left_line1 = Line::from(vec![
        Span::styled(
            format!(" {} {}", state_icon, match app.playback.state {
                PlayState::Playing => "Playing",
                PlayState::Paused => "Paused",
                PlayState::Stopped => "Stopped",
            }),
            Style::default().fg(state_color).add_modifier(Modifier::BOLD),
        ),
    ]);

    let left_line2 = Line::from(vec![
        Span::styled(
            format!(" {}/{}{}", pos, dur, bitrate),
            Style::default().fg(Color::Gray),
        ),
    ]);

    let left = Paragraph::new(vec![left_line1, left_line2]);
    frame.render_widget(left, cols[0]);

    // Center: Track info
    let (title, artist_album) = if let Some(track) = app.current_track() {
        (
            track.title.clone(),
            format!("{} - {}", track.display_artist(), track.display_album()),
        )
    } else {
        ("No track playing".to_string(), String::new())
    };

    let center_line1 = Line::from(Span::styled(
        title,
        Style::default().fg(Color::Rgb(100, 220, 255)).add_modifier(Modifier::BOLD),
    )).alignment(Alignment::Center);

    let center_line2 = Line::from(Span::styled(
        artist_album,
        Style::default().fg(Color::Rgb(200, 170, 255)),
    )).alignment(Alignment::Center);

    let center = Paragraph::new(vec![center_line1, center_line2]);
    frame.render_widget(center, cols[1]);

    // Right: Volume + shuffle/repeat
    let vol_pct = (app.playback.volume * 100.0) as u8;

    let shuffle_style = if app.playback.shuffle {
        Style::default().fg(Color::Rgb(100, 220, 255)).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let repeat_style = match app.playback.repeat {
        crate::app::state::RepeatMode::Off => Style::default().fg(Color::DarkGray),
        crate::app::state::RepeatMode::All => Style::default().fg(Color::Rgb(120, 255, 180)).add_modifier(Modifier::BOLD),
        crate::app::state::RepeatMode::One => Style::default().fg(Color::Rgb(255, 220, 100)).add_modifier(Modifier::BOLD),
    };

    // Volume staircase with gradient: green → yellow → orange → red
    const STEPS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let vol_colors: [Color; 8] = [
        Color::Rgb(80, 200, 120),  // green
        Color::Rgb(120, 220, 100), // green-yellow
        Color::Rgb(180, 230, 80),  // yellow-green
        Color::Rgb(230, 220, 60),  // yellow
        Color::Rgb(255, 190, 50),  // amber
        Color::Rgb(255, 150, 40),  // orange
        Color::Rgb(255, 110, 50),  // red-orange
        Color::Rgb(255, 70, 70),   // red
    ];
    let filled = (vol_pct as u16 * 8 / 100).min(8) as usize;
    let mut vol_spans = Vec::with_capacity(10);
    for (i, &ch) in STEPS.iter().enumerate() {
        let style = if i < filled {
            Style::default().fg(vol_colors[i]).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Indexed(238))
        };
        vol_spans.push(Span::styled(String::from(ch), style));
    }
    vol_spans.push(Span::styled(format!(" {}% ", vol_pct), Style::default().fg(Color::White)));

    let right_line1 = Line::from(vol_spans).alignment(Alignment::Right);

    let is_bookmarked = app.queue.current_index
        .and_then(|qi| app.queue.tracks.get(qi))
        .is_some_and(|&ti| {
            app.playlists.iter().any(|pl| pl.tracks.contains(&ti))
        });
    let bookmark_style = if is_bookmarked {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let right_line2 = Line::from(vec![
        Span::styled("\u{F005} ", bookmark_style),  // nf-fa-star
        Span::styled("\u{F074} ", shuffle_style),   // nf-fa-random
        Span::styled(format!("{} ", app.playback.repeat.symbol()), repeat_style),
    ]).alignment(Alignment::Right);

    let right = Paragraph::new(vec![right_line1, right_line2]);
    frame.render_widget(right, cols[2]);
}

fn format_time(secs: f64) -> String {
    let total = secs as u64;
    let m = total / 60;
    let s = total % 60;
    format!("{}:{:02}", m, s)
}
