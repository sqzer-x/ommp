use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub fg: Color,
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
    pub tab_inactive: Style,
    pub progress_filled: Color,
    pub progress_empty: Color,
    pub dim_style: Style,
    pub current_track_style: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            fg: Color::White,
            // These were both Cyan, which made the focus indicator a no-op:
            // every pane picked a border colour from `focused` and got the same
            // answer, so there was no way to see where focus was.
            border_focused: Color::Rgb(100, 220, 255),
            border_unfocused: Color::Rgb(70, 80, 95),
            highlight_bg: Color::Cyan,
            highlight_fg: Color::Black,
            tab_inactive: Style::default().fg(Color::DarkGray),
            progress_filled: Color::Rgb(200, 80, 255),
            progress_empty: Color::Indexed(236),
            dim_style: Style::default().fg(Color::DarkGray),
            current_track_style: Style::default()
                .fg(Color::Rgb(100, 220, 255))
                .add_modifier(Modifier::BOLD),
        }
    }
}
