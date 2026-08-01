//! Small formatting and layout helpers shared across the UI.
//!
//! Each of these existed as two to four byte-identical copies scattered through
//! the panes and modals.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Truncate `s` to `max_width` terminal columns, appending "…" when it does not
/// fit, and pad with spaces so the result occupies exactly `max_width`.
///
/// Column-aware rather than byte- or char-aware: a CJK title is twice as wide as
/// its character count, so padding by length misaligns every following column.
pub fn fit_to_width(s: &str, max_width: usize) -> String {
    let str_width = UnicodeWidthStr::width(s);
    if str_width <= max_width {
        return format!("{}{}", s, " ".repeat(max_width - str_width));
    }

    let mut w = 0;
    let mut result = String::new();
    for ch in s.chars() {
        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
        // Reserve one column for the ellipsis.
        if w + ch_w + 1 > max_width {
            result.push('\u{2026}');
            w += 1;
            break;
        }
        w += ch_w;
        result.push(ch);
    }
    result.push_str(&" ".repeat(max_width.saturating_sub(w)));
    result
}

/// Seconds as `m:ss`.
pub fn format_time(secs: f64) -> String {
    let total = secs.max(0.0) as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

/// A rect of the given percentage size, centred in `area`.
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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

/// Largest scroll offset that still fills the viewport.
///
/// The panes each clamped to `count - 1`, which let the list scroll until a
/// single row sat above a screen of blank space.
pub fn max_scroll(count: usize, viewport: usize) -> usize {
    count.saturating_sub(viewport)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_pads_and_truncates_by_column_not_by_character() {
        assert_eq!(fit_to_width("abc", 5), "abc  ");
        assert_eq!(UnicodeWidthStr::width(fit_to_width("abcdefgh", 5).as_str()), 5);
        // Four wide characters are eight columns; two fit alongside the ellipsis.
        assert_eq!(fit_to_width("日本語テスト", 5), "日本\u{2026}");
        assert_eq!(UnicodeWidthStr::width(fit_to_width("日本語テスト", 5).as_str()), 5);
        // A wide string that exactly fits is left alone.
        assert_eq!(fit_to_width("肺", 2), "肺");
    }

    #[test]
    fn max_scroll_stops_before_the_list_runs_out() {
        // 100 items in a 30-row viewport: the last full screen starts at 70.
        assert_eq!(max_scroll(100, 30), 70);
        // Everything already visible means no scrolling.
        assert_eq!(max_scroll(10, 30), 0);
        assert_eq!(max_scroll(0, 30), 0);
    }

    #[test]
    fn format_time_pads_seconds() {
        assert_eq!(format_time(0.0), "0:00");
        assert_eq!(format_time(65.9), "1:05");
        assert_eq!(format_time(-1.0), "0:00");
    }
}
