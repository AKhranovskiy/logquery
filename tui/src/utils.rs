use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// # Usage
///
/// ```rust
/// let rect = centered_rect(f.size(), 50, 50);
/// ```
pub fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn now() -> time::OffsetDateTime {
    time::OffsetDateTime::now_utc().to_offset(time::macros::offset!(+07))
}

pub fn file_name(path: &std::path::Path) -> Option<String> {
    path.iter()
        .last()
        .map(std::ffi::OsStr::to_string_lossy)
        .as_ref()
        .map(std::borrow::Cow::to_string)
}
