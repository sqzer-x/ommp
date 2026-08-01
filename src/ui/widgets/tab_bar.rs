use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::state::Tab;
use crate::ui::theme::Theme;

pub fn render_tab_bar(
    frame: &mut Frame,
    area: Rect,
    current: Tab,
    hovered: Option<usize>,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_unfocused));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // One accent colour per tab. This was `[Color; 6]` against seven tabs, so
    // the modulo wrapped and Playlists rendered in Queue's sky blue — and every
    // comment from index 5 named the wrong tab.
    let tab_colors: [Color; Tab::ALL.len()] = [
        Color::Rgb(100, 220, 255), // Queue - sky blue
        Color::Rgb(120, 255, 180), // Directories - mint green
        Color::Rgb(255, 180, 100), // Artists - orange
        Color::Rgb(200, 130, 255), // Albums - purple
        Color::Rgb(255, 120, 150), // Genre - pink
        Color::Rgb(120, 200, 255), // Format - steel blue
        Color::Rgb(255, 220, 100), // Playlists - gold
    ];

    let mut spans = Vec::new();
    for (i, tab) in Tab::ALL.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" \u{2502} ", theme.dim_style));
        }
        let color = tab_colors[i];
        let style = if i == current.index() {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else if hovered == Some(i) {
            Style::default().fg(color)
        } else {
            theme.tab_inactive
        };
        spans.push(Span::styled(tab.title(), style));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}

/// Which tab, if any, sits under the pointer.
///
/// `mouse_y` matters: only the x range was checked, so clicking the tab bar's
/// top or bottom border switched tabs.
pub fn tab_hit_test(area: Rect, mouse_x: u16, mouse_y: u16) -> Option<usize> {
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);

    if mouse_x < inner.x
        || mouse_x >= inner.x + inner.width
        || mouse_y < inner.y
        || mouse_y >= inner.y + inner.height
    {
        return None;
    }

    // Calculate total content width to find center offset
    let divider_len = 3; // " │ "
    let mut total_width: usize = 0;
    for (i, tab) in Tab::ALL.iter().enumerate() {
        if i > 0 {
            total_width += divider_len;
        }
        total_width += tab.title().len();
    }

    let inner_w = inner.width as usize;
    let pad_left = if inner_w > total_width {
        (inner_w - total_width) / 2
    } else {
        0
    };

    let rel_x = (mouse_x - inner.x) as usize;
    if rel_x < pad_left {
        return None;
    }

    let content_x = rel_x - pad_left;
    let mut pos = 0;
    for (i, tab) in Tab::ALL.iter().enumerate() {
        if i > 0 {
            // Skip the divider rather than folding it into the next tab: the
            // three columns of " │ " used to select whichever tab followed.
            if content_x < pos + divider_len {
                return None;
            }
            pos += divider_len;
        }
        let title_len = tab.title().len();
        if content_x < pos + title_len {
            return Some(i);
        }
        pos += title_len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wide enough that the tab strip is not padded off-centre.
    fn bar() -> Rect {
        Rect::new(0, 0, 200, 3)
    }

    #[test]
    fn borders_are_not_part_of_any_tab() {
        // Row 0 and row 2 are the block's borders; row 1 is the content.
        assert!(tab_hit_test(bar(), 100, 0).is_none());
        assert!(tab_hit_test(bar(), 100, 2).is_none());
        assert!(tab_hit_test(bar(), 100, 1).is_some());
    }

    #[test]
    fn dividers_belong_to_neither_neighbour() {
        let inner_x = 1;
        let pad = (198 - total_content_width()) / 2;
        let queue_len = Tab::ALL[0].title().len();
        // The three divider columns sit right after "Queue".
        for offset in 0..3 {
            let x = inner_x + pad + queue_len + offset;
            assert_eq!(tab_hit_test(bar(), x as u16, 1), None, "offset {offset}");
        }
        // The tab after the divider is still reachable.
        let x = inner_x + pad + queue_len + 3;
        assert_eq!(tab_hit_test(bar(), x as u16, 1), Some(1));
    }

    fn total_content_width() -> usize {
        Tab::ALL.iter().map(|t| t.title().len()).sum::<usize>() + 3 * (Tab::ALL.len() - 1)
    }
}
