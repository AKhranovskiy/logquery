use std::{
    cmp::Ordering,
    fmt::{Display, Write},
    hash::{DefaultHasher, Hash, Hasher},
};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use itertools::Itertools;
use ratatui::{
    layout::Constraint,
    prelude::{Buffer, Rect},
    style::{Style, Stylize},
    text::Text,
    widgets::{Block, Borders, HighlightSpacing, Row, StatefulWidget, Table, TableState},
};
use time::macros::format_description;

use crate::{
    repository::FileInfo,
    utils::{self, centered_rect},
};

const WIDTHS: [Constraint; 4] = [
    Constraint::Fill(1),    // File name
    Constraint::Length(8),  // Number of lines
    Constraint::Length(8),  // Age
    Constraint::Length(20), // Last update
];

const LABELS: [&str; 4] = ["Name", "Lines", "Age", "Last update"];
const TITLE: &str = "File browser";

const LAST_UPDATE_FORMAT: &[time::format_description::BorrowedFormatItem<'_>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

pub struct FileBrowser {}

#[derive(Debug, Clone)]
pub struct FileBrowserState {
    hash: u64,
    sorted_list: Vec<FileInfo>,
    sort_column: SortColumn,
    sort_direction: SortDirection,
    table_state: TableState,
    visible: bool,
}

impl FileBrowserState {
    pub fn new() -> Self {
        let hash = {
            let mut h = DefaultHasher::new();
            Vec::<FileInfo>::new().hash(&mut h);
            h.finish()
        };

        Self {
            hash,
            sorted_list: Vec::new(),
            sort_column: SortColumn::Name,
            sort_direction: SortDirection::Ascending,
            table_state: TableState::default(),
            visible: true,
        }
    }

    #[allow(unreachable_code)]
    pub fn handle_key_event(&mut self, event: &KeyEvent) {
        if self.visible {
            match (event.kind, event.code) {
                // File list table sorting
                (KeyEventKind::Press, KeyCode::Char('n')) => {
                    self.sort_column = SortColumn::Name;
                    self.sort_direction = SortDirection::Ascending;
                }
                (KeyEventKind::Press, KeyCode::Char('N')) => {
                    self.sort_column = SortColumn::Name;
                    self.sort_direction = SortDirection::Descending;
                }
                (KeyEventKind::Press, KeyCode::Char('l')) => {
                    self.sort_column = SortColumn::LineCount;
                    self.sort_direction = SortDirection::Ascending;
                }
                (KeyEventKind::Press, KeyCode::Char('L')) => {
                    self.sort_column = SortColumn::LineCount;
                    self.sort_direction = SortDirection::Descending;
                }
                (KeyEventKind::Press, KeyCode::Char('a')) => {
                    self.sort_column = SortColumn::Age;
                    self.sort_direction = SortDirection::Ascending;
                }
                (KeyEventKind::Press, KeyCode::Char('A')) => {
                    self.sort_column = SortColumn::Age;
                    self.sort_direction = SortDirection::Descending;
                }

                // File list selection
                (KeyEventKind::Press, KeyCode::Up) => {
                    self.table_state
                        .select(self.table_state.selected().map(|v| v.saturating_sub(1)));
                }
                (KeyEventKind::Press, KeyCode::Down) => {
                    self.table_state.select(
                        self.table_state
                            .selected()
                            .map(|v| v.saturating_add(1).min(self.sorted_list.len() - 1)),
                    );
                }

                // File list actions
                (KeyEventKind::Press, KeyCode::Enter) => {
                    match event.modifiers {
                        KeyModifiers::SHIFT => {
                            // Try to split into the current tab, otherwise new tab
                            unimplemented!("Split into the current tab");
                        }
                        KeyModifiers::CONTROL => {
                            // Try to merge into the current tab, otherwise new tab
                            unimplemented!("Merge into the current tab");
                        }
                        _ => {
                            unimplemented!("Open tab for selected file");
                        }
                    }
                    self.visible = false;
                }
                _ => {}
            }
        } else if (event.kind, event.code) == (KeyEventKind::Press, KeyCode::Char('o')) {
            self.visible = true;
        }
    }

    pub fn update(&mut self, files: Vec<FileInfo>) {
        if self.visible {
            let hash = {
                let mut h = DefaultHasher::new();
                files.hash(&mut h);
                h.finish()
            };
            if self.hash != hash {
                self.sorted_list = sort(files, self.sort_column, self.sort_direction);

                if self.sorted_list.is_empty() {
                    self.table_state.select(None);
                } else {
                    self.table_state.select(
                        self.table_state
                            .selected()
                            .map(|v| v.min(self.sorted_list.len() - 1))
                            .or(Some(0)),
                    );
                }
            }
        }
    }

    pub const fn is_visible(&self) -> bool {
        self.visible
    }
}

struct Renderer<'state>(&'state FileBrowserState);

impl<'state> Renderer<'state> {
    fn header(&self) -> Row<'state> {
        Row::new(vec![
            Text::from(format_label(
                LABELS[0],
                self.0.sort_column == SortColumn::Name,
                self.0.sort_direction,
            ))
            .left_aligned(),
            Text::from(format_label(
                LABELS[1],
                self.0.sort_column == SortColumn::LineCount,
                self.0.sort_direction,
            ))
            .right_aligned(),
            Text::from(format_label(
                LABELS[2],
                self.0.sort_column == SortColumn::Age,
                self.0.sort_direction,
            ))
            .right_aligned(),
            Text::from(LABELS[3]).left_aligned(),
        ])
    }

    fn rows(&self) -> Vec<Row<'state>> {
        self.0
            .sorted_list
            .iter()
            .map(|file| {
                let age = (utils::now() - file.last_update).whole_seconds();
                let last_update = file.last_update.format(LAST_UPDATE_FORMAT).unwrap();

                Row::new(vec![
                    Text::from(file.name.clone()).left_aligned(),
                    Text::from(file.number_of_lines.to_string()).right_aligned(),
                    Text::from(age.to_string()).right_aligned(),
                    Text::from(last_update).left_aligned(),
                ])
            })
            .collect_vec()
    }
}

impl FileBrowser {}

impl StatefulWidget for FileBrowser {
    type State = FileBrowserState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let renderer = Renderer(state);

        let area = centered_rect(area, 60, 80);

        let table = Table::new(renderer.rows(), WIDTHS)
            .block(Block::default().title(TITLE).borders(Borders::ALL))
            .header(renderer.header())
            .highlight_spacing(HighlightSpacing::Always)
            .highlight_style(Style::default().bold().yellow().on_blue());

        let mut table_state = state.table_state.clone();
        StatefulWidget::render(table, area, buf, &mut table_state);
        state.table_state = table_state;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortColumn {
    Age,
    LineCount,
    Name,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortDirection {
    Ascending,
    Descending,
}

impl From<SortDirection> for char {
    fn from(direction: SortDirection) -> Self {
        match direction {
            SortDirection::Ascending => '▼',
            SortDirection::Descending => '▲',
        }
    }
}

impl Display for SortDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char(char::from(*self))
    }
}

fn sort(files: Vec<FileInfo>, column: SortColumn, direction: SortDirection) -> Vec<FileInfo> {
    let cmp = match column {
        SortColumn::Name => FileInfoExt::cmp_by_name,
        SortColumn::Age => FileInfoExt::cmp_by_age,
        SortColumn::LineCount => FileInfoExt::cmp_by_line_count,
    };

    let sorted = files.into_iter().sorted_by(cmp);

    match direction {
        SortDirection::Ascending => sorted.collect(),
        SortDirection::Descending => sorted.rev().collect(),
    }
}

trait FileInfoExt {
    fn cmp_by_name(&self, other: &Self) -> Ordering;
    fn cmp_by_age(&self, other: &Self) -> Ordering;
    fn cmp_by_line_count(&self, other: &Self) -> Ordering;
}

impl FileInfoExt for FileInfo {
    fn cmp_by_name(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }

    fn cmp_by_age(&self, other: &Self) -> Ordering {
        self.last_update.cmp(&other.last_update).reverse()
    }

    fn cmp_by_line_count(&self, other: &Self) -> Ordering {
        self.number_of_lines.cmp(&other.number_of_lines)
    }
}

fn format_label(label: &str, sorted: bool, direction: SortDirection) -> String {
    if sorted {
        format!("{label} {direction}")
    } else {
        label.to_string()
    }
}
