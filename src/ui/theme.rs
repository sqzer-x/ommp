use ratatui::style::{Color, Modifier, Style};

#[allow(dead_code)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
    pub tab_active: Style,
    pub tab_inactive: Style,
    pub status_bar_bg: Color,
    pub progress_filled: Color,
    pub progress_empty: Color,
    pub playing_indicator: Color,
    pub title_style: Style,
    pub artist_style: Style,
    pub dim_style: Style,
    pub current_track_style: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::White,
            border_focused: Color::Cyan,
            border_unfocused: Color::Cyan,
            highlight_bg: Color::Cyan,
            highlight_fg: Color::Black,
            tab_active: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(Color::DarkGray),
            status_bar_bg: Color::DarkGray,
            progress_filled: Color::Rgb(200, 80, 255),
            progress_empty: Color::Indexed(236),
            playing_indicator: Color::Rgb(80, 255, 120),
            title_style: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            artist_style: Style::default().fg(Color::Gray),
            dim_style: Style::default().fg(Color::DarkGray),
            current_track_style: Style::default()
                .fg(Color::Rgb(100, 220, 255))
                .add_modifier(Modifier::BOLD),
        }
    }
}
