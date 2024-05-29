use std::{
    cmp::Ordering,
    fmt::{Display, Write},
    hash::{DefaultHasher, Hash, Hasher},
};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use itertools::Itertools;
use ratatui::{
    layout::Constraint,
    prelude::{Buffer, Rect},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Borders, HighlightSpacing, Row, StatefulWidget, Table, TableState},
};
use time::macros::format_description;

use crate::{
    repository::{FileInfo, RepoList},
    utils::{self, centered_rect},
};

use super::KeyEventHandler;

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

#[derive(Debug, Clone, Copy)]
pub struct FileList {}

#[derive(Debug, Default, Clone)]
pub struct FileListState {
    hash: u64,
    sorted_list: Vec<FileInfo>,
    sort_column: SortColumn,
    sort_direction: SortDirection,
    table_state: TableState,
}

impl KeyEventHandler for FileListState {
    type Action = FileInfo;

    fn handle_key_event(&mut self, event: &KeyEvent) -> Option<Self::Action> {
        if let Some(selected) = self.selected() {
            if (KeyEventKind::Press, KeyCode::Enter) == (event.kind, event.code) {
                return selected.into();
            }
        }

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
                self.table_state
                    .select(self.table_state.selected().map(|v| {
                        v.saturating_add(1)
                            .min(self.sorted_list.len().saturating_sub(1))
                    }));
            }

            _ => {}
        }

        None
    }
}

impl FileListState {
    pub fn update(&mut self, repo: &impl RepoList) {
        let files = repo.list();

        let hash = {
            let mut h = DefaultHasher::new();
            files.hash(&mut h);
            h.finish()
        };

        if self.hash == hash {
            return;
        }

        let index = self
            .table_state
            .selected()
            .and_then(|s| self.sorted_list.get(s))
            .map(|info| info.name.clone());

        self.sorted_list = sort(files, self.sort_column, self.sort_direction);

        let index =
            index.and_then(|name| self.sorted_list.iter().position(|info| info.name == name));

        self.table_state.select(index.or(Some(0)));
    }

    fn selected(&self) -> Option<FileInfo> {
        self.sorted_list.get(self.table_state.selected()?).cloned()
    }
}

struct Renderer<'state>(&'state FileListState);

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
        .bottom_margin(1)
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
                    Text::from(Line::from_iter([age.to_string(), "s".into()])).right_aligned(),
                    Text::from(last_update).left_aligned(),
                ])
            })
            .collect_vec()
    }
}

impl FileList {}

impl StatefulWidget for FileList {
    type State = FileListState;

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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum SortColumn {
    Age,
    LineCount,
    #[default]
    Name,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum SortDirection {
    #[default]
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
