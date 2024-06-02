use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use itertools::Itertools;
use ratatui::{
    prelude::*,
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Tabs,
    },
};

use crate::repository::{FileInfo, RepoLines};

use super::KeyEventHandler;

struct FileState {
    pub name: String,
    total_lines: u32,
    number_column_width: u16,
    scroll_offset: u32,
    display_lines: Box<[Arc<str>]>,
}

impl From<FileInfo> for FileState {
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
            display_lines: Box::default(),
        }
    }
}

#[derive(Default)]
pub struct FileViewState {
    height: u32,
    files: Vec<FileState>,
    active: usize,
}

impl KeyEventHandler for FileViewState {
    type Action = ();

    fn handle_key_event(&mut self, event: &KeyEvent) -> Option<Self::Action> {
        let active = self.files.get_mut(self.active)?;

        let with_shift = event.modifiers.contains(KeyModifiers::SHIFT);

        match (event.kind, event.code) {
            (KeyEventKind::Press, KeyCode::Up) => {
                active.scroll_offset = if with_shift {
                    active.scroll_offset.saturating_sub(self.height)
                } else {
                    active.scroll_offset.saturating_sub(1)
                };
            }
            (KeyEventKind::Press, KeyCode::Down) => {
                active.scroll_offset = if with_shift {
                    active.scroll_offset.saturating_add(self.height)
                } else {
                    active.scroll_offset.saturating_add(1)
                }
                .min(active.total_lines.saturating_sub(self.height));
            }
            (KeyEventKind::Press, KeyCode::PageUp) => {
                active.scroll_offset = active.scroll_offset.saturating_sub(self.height);
            }
            (KeyEventKind::Press, KeyCode::PageDown) => {
                active.scroll_offset = active
                    .scroll_offset
                    .saturating_add(self.height)
                    .min(active.total_lines.saturating_sub(self.height));
            }
            _ => {}
        }

        None
    }
}

impl FileViewState {
    pub fn push(&mut self, info: FileInfo) {
        if let Some(pos) = self.files.iter().position(|state| state.name == info.name) {
            self.active = pos;
        } else {
            self.files.push(info.into());
            self.active = self.files.len() - 1;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    // pub fn len(&self) -> usize {
    //     self.files.len()
    // }

    pub fn update(&mut self, repo: &impl RepoLines) {
        if let Some(state) = self.files.get_mut(self.active) {
            state.display_lines = repo.lines(
                state.name.as_str(),
                state.scroll_offset,
                (state.scroll_offset + self.height).min(state.total_lines),
            );
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FileView {}

impl StatefulWidget for FileView {
    type State = FileViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Update the visible lines count
        state.height = area.height.saturating_sub(3).into();

        let frame_height = state.height;

        let tab_titles = state
            .files
            .iter()
            .map(|state| state.name.clone())
            .collect_vec();

        let Some(active_state) = state.files.get_mut(state.active) else {
            return;
        };

        let layout = FileViewLayout::new(area, active_state.number_column_width);

        // Tabs
        {
            Tabs::new(tab_titles)
                .highlight_style(Style::default().bold().yellow())
                .padding("", "")
                .divider(" ")
                .select(state.active)
                .render(layout.tabs, buf);
        }

        // Numbers column
        {
            let line_numbers = ((active_state.scroll_offset)
                ..(active_state.scroll_offset + frame_height))
                .map(|i| {
                    Line::from(vec![Span::raw((i + 1).to_string()), Span::raw(" ")])
                        .right_aligned()
                        .dark_gray()
                })
                .collect_vec();

            let column = Paragraph::new(line_numbers).block(
                Block::new()
                    .borders(Borders::TOP | Borders::BOTTOM)
                    .border_style(Style::default().dark_gray()),
            );

            Widget::render(column, layout.numbers, buf);
        }

        // Text area
        {
            let lines = active_state
                .display_lines
                .iter()
                .map(|line| Line::from(line.as_ref()))
                .collect_vec();

            // Use custom border set to merge [Numbers] and [Text] bottom borders.
            let border_set = symbols::border::Set {
                bottom_left: symbols::line::NORMAL.horizontal_up,
                top_left: symbols::line::NORMAL.horizontal_down,
                ..symbols::border::PLAIN
            };

            let par = Paragraph::new(lines).block(
                Block::new()
                    .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)
                    .border_style(Style::default().dark_gray())
                    .border_set(border_set),
            );

            Widget::render(par, layout.text, buf);
        }

        // Top-right corner
        {
            let block = Block::new()
                .borders(Borders::TOP | Borders::RIGHT)
                .border_style(Style::default().dark_gray());

            Widget::render(block, layout.top_right_corner, buf);
        }

        // Scrollbar
        {
            if active_state.total_lines > frame_height {
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol("│".into())
                    .thumb_symbol("┃");

                let mut scrollbar_state =
                    ScrollbarState::new(active_state.total_lines.saturating_sub(frame_height) as _)
                        .position(active_state.scroll_offset as _);

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
    tabs: Rect,
    numbers: Rect,
    text: Rect,
    top_right_corner: Rect,
    scrollbar: Rect,
    bottom_right_corner: Rect,
}

/// Layout of the file view
///  ```
/// [          tabs       ]
/// [numbers][text] [scrollbar]
///                 [bottom_right_corner]
/// ```
impl FileViewLayout {
    fn new(area: Rect, number_column_width: u16) -> Self {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)])
            .split(area);

        let tabs = layout[0];

        let main = Layout::horizontal(vec![
            Constraint::Length(number_column_width),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(layout[1]);

        let numbers = main[0];
        let text = main[1];

        // Split scrollbar area into 3 rows because Scrollbar cannot be wrapped with a borderd block.
        let scrollbar_area = Layout::vertical(vec![
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(main[2]);

        let top_right_corner = scrollbar_area[0];
        let scrollbar = scrollbar_area[1];
        let bottom_right_corner = scrollbar_area[2];

        Self {
            tabs,
            numbers,
            text,
            top_right_corner,
            scrollbar,
            bottom_right_corner,
        }
    }
}
