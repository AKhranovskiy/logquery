use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};

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

pub trait KeyEventExt {
    fn has_pressed(&self, c: char) -> bool;
}

impl KeyEventExt for event::KeyEvent {
    fn has_pressed(&self, c: char) -> bool {
        (self.kind, self.code) == (KeyEventKind::Press, KeyCode::Char(c))
    }
}

pub trait RectExt {
    fn outer(self, mar: Margin) -> Self;
    fn inner_centered(self, percent_x: u16, percent_y: u16) -> Self;
}

impl RectExt for Rect {
    fn outer(self, mar: Margin) -> Self {
        Self {
            x: self.x.saturating_sub(mar.horizontal),
            y: self.y.saturating_sub(mar.vertical),
            width: self.width.saturating_add(mar.horizontal * 2),
            height: self.height.saturating_add(mar.vertical * 2),
        }
    }

    fn inner_centered(self, percent_x: u16, percent_y: u16) -> Self {
        assert!(percent_x <= 100);
        assert!(percent_y <= 100);

        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(self)[1];

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(area)[1]
    }
}
