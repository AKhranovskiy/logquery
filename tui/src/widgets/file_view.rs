use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use itertools::Itertools;
use ratatui::{
    prelude::*,
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
    },
};

use crate::repository::{FileInfo, RepoLines};

use super::KeyEventHandler;

pub struct FileViewState {
    name: String,
    total_lines: u32,
    number_column_width: u16,
    scroll_offset: u32,
    frame_height: u32,
    display_lines: Box<[Arc<str>]>,
}

impl From<FileInfo> for FileViewState {
    fn from(info: FileInfo) -> Self {
        Self {
            name: info.name,
            total_lines: info.number_of_lines,
            number_column_width: info
                .number_of_lines
                .to_string()
                .len()
                .try_into()
                .unwrap_or(1u16)
                + 3,
            scroll_offset: 0,
            frame_height: 0, // will be updated on render
            display_lines: Box::default(),
        }
    }
}

impl FileViewState {
    pub fn update(&mut self, repo: &impl RepoLines) {
        self.display_lines = repo.lines(
            self.name.as_str(),
            self.scroll_offset,
            (self.scroll_offset + self.frame_height).min(self.total_lines),
        );
    }
}

impl KeyEventHandler for FileViewState {
    type Action = ();

    fn handle_key_event(&mut self, event: &KeyEvent) -> Option<Self::Action> {
        let with_shift = event.modifiers.contains(KeyModifiers::SHIFT);

        match (event.kind, event.code) {
            (KeyEventKind::Press, KeyCode::Up) => {
                self.scroll_offset = if with_shift {
                    self.scroll_offset.saturating_sub(self.frame_height)
                } else {
                    self.scroll_offset.saturating_sub(1)
                };
            }
            (KeyEventKind::Press, KeyCode::Down) => {
                self.scroll_offset = if with_shift {
                    self.scroll_offset.saturating_add(self.frame_height)
                } else {
                    self.scroll_offset.saturating_add(1)
                }
                .min(self.total_lines.saturating_sub(self.frame_height));
            }
            (KeyEventKind::Press, KeyCode::PageUp) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(self.frame_height);
            }
            (KeyEventKind::Press, KeyCode::PageDown) => {
                self.scroll_offset = self
                    .scroll_offset
                    .saturating_add(self.frame_height)
                    .min(self.total_lines.saturating_sub(self.frame_height));
            }
            _ => {}
        }
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FileView {}

impl StatefulWidget for FileView {
    type State = FileViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Update the visible lines count
        state.frame_height = area.height.into();
        state.frame_height = state.frame_height.saturating_sub(2);

        let layout = FileViewLayout::new(area, state.number_column_width);

        // Top-left corner
        {
            let block = Block::new()
                .borders(Borders::TOP)
                .border_style(Style::default().dark_gray());
            Widget::render(block, layout.top_left_corner, buf);
        }

        // Title
        {
            // Use custom border set to merge [TopLeftCorner] and [Title] top borders.
            let border_set = symbols::border::Set {
                top_left: symbols::line::NORMAL.horizontal_down,
                ..symbols::border::PLAIN
            };
            let block = Block::new()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .border_set(border_set)
                .border_style(Style::default().dark_gray())
                .title(Line::from(state.name.as_ref()).bold().yellow());

            Widget::render(block, layout.title, buf);
        }

        // Numbers column
        {
            let line_numbers = ((state.scroll_offset)..(state.scroll_offset + state.frame_height))
                .map(|i| {
                    Line::from(vec![Span::raw((i + 1).to_string()), Span::raw(" ")])
                        .right_aligned()
                        .dark_gray()
                })
                .collect_vec();

            let column = Paragraph::new(line_numbers).block(
                Block::new()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().dark_gray()),
            );
            Widget::render(column, layout.numbers, buf);
        }

        // Text area
        {
            let lines = state
                .display_lines
                .iter()
                .map(|line| Line::from(line.as_ref()))
                .collect_vec();

            // Use custom border set to merge [Numbers] and [Text] bottom borders.
            let border_set = symbols::border::Set {
                bottom_left: symbols::line::NORMAL.horizontal_up,
                ..symbols::border::PLAIN
            };

            let par = Paragraph::new(lines).block(
                Block::new()
                    .borders(Borders::LEFT | Borders::BOTTOM)
                    .border_style(Style::default().dark_gray())
                    .border_set(border_set),
            );

            Widget::render(par, layout.text, buf);
        }

        // Scrollbar
        {
            if state.total_lines > state.frame_height {
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol("│".into())
                    .thumb_symbol("┃");

                let mut scrollbar_state =
                    ScrollbarState::new(state.total_lines.saturating_sub(state.frame_height) as _)
                        .position(state.scroll_offset as _);

                StatefulWidget::render(scrollbar, layout.scrollbar, buf, &mut scrollbar_state);
            } else {
                let block = Block::new()
                    .borders(Borders::RIGHT)
                    .border_style(Style::default().dark_gray());
                Widget::render(block, layout.scrollbar, buf);
            }
        }

        // Bottom-right corner
        {
            let block = Block::new()
                .borders(Borders::BOTTOM | Borders::RIGHT)
                .border_style(Style::default().dark_gray());
            Widget::render(block, layout.bottom_right_corner, buf);
        }
    }
}

struct FileViewLayout {
    top_left_corner: Rect,
    title: Rect,
    numbers: Rect,
    text: Rect,
    scrollbar: Rect,
    bottom_right_corner: Rect,
}

/// Layout of the file view
///  ```
/// [top_left_corner] [title]
/// [numbers]         [text] [scrollbar]
///                          [bottom_right_corner]
/// ```
impl FileViewLayout {
    fn new(area: Rect, number_column_width: u16) -> Self {
        let vert = Layout::vertical(vec![Constraint::Length(1), Constraint::Fill(1)]).split(area);

        let top_area = Layout::horizontal(vec![
            Constraint::Length(number_column_width),
            Constraint::Fill(1),
        ])
        .split(vert[0]);
        let top_left_corner = top_area[0];
        let title = top_area[1];

        let main_area = Layout::horizontal(vec![
            Constraint::Length(number_column_width),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(vert[1]);
        let numbers = main_area[0];
        let text = main_area[1];

        // Split scrollbar area into two rows because Scrollbar cannot be wrapped with a borderd block.
        let scrollbar_area =
            Layout::vertical(vec![Constraint::Fill(1), Constraint::Length(1)]).split(main_area[2]);
        let scrollbar = scrollbar_area[0];
        let bottom_right_corner = scrollbar_area[1];

        Self {
            top_left_corner,
            title,
            numbers,
            text,
            scrollbar,
            bottom_right_corner,
        }
    }
}
