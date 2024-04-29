use itertools::Itertools;
use ratatui::{
    layout::Constraint,
    prelude::{Buffer, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, HighlightSpacing, Row, StatefulWidget, Table, TableState},
};
use time::macros::format_description;

use crate::{
    repository::{FileInfo, FileInfoSortKey, SortDirection},
    utils::{self, centered_rect},
};

pub struct FileInfoTable<'a> {
    header: Row<'a>,
    widths: Vec<Constraint>,
    rows: Vec<Row<'a>>,
    #[allow(dead_code)]
    sort: (FileInfoSortKey, SortDirection),
}

fn sort_marker(sorted: bool, direction: SortDirection) -> String {
    if sorted {
        format!(" {direction}")
    } else {
        String::default()
    }
}

impl<'a> FileInfoTable<'a> {
    pub fn new(
        files: &'a [FileInfo],
        sort_key: FileInfoSortKey,
        sort_direction: SortDirection,
    ) -> Self {
        let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

        let widths = vec![
            Constraint::Fill(1),    // File name
            Constraint::Length(8),  // Number of lines
            Constraint::Length(8),  // Age
            Constraint::Length(20), // Last update
        ];

        let sorted_by_name = sort_marker(sort_key == FileInfoSortKey::Name, sort_direction);
        let sorted_by_line_count =
            sort_marker(sort_key == FileInfoSortKey::LineCount, sort_direction);
        let sorted_by_age = sort_marker(sort_key == FileInfoSortKey::LastUpdate, sort_direction);

        let header = Row::new(vec![
            Text::from(Line::from(vec![
                Span::raw("Name"),
                Span::raw(sorted_by_name),
            ]))
            .left_aligned(),
            Text::from(Line::from(vec![
                Span::raw("Lines"),
                Span::raw(sorted_by_line_count),
            ]))
            .right_aligned(),
            Text::from(Line::from(vec![Span::raw("Age"), Span::raw(sorted_by_age)]))
                .right_aligned(),
            Text::from("Last update").left_aligned(),
        ])
        .style(Style::default().bold())
        .bottom_margin(1);

        let rows = files
            .iter()
            .map(|file| {
                Row::new(vec![
                    Text::from(file.name.as_str()).left_aligned(),
                    Text::from(file.number_of_lines.to_string()).right_aligned(),
                    Text::from(format!(
                        "{}s",
                        (utils::now() - file.last_update).whole_seconds()
                    ))
                    .right_aligned(),
                    Text::from(file.last_update.format(&format).unwrap_or_default()).left_aligned(),
                ])
            })
            .collect_vec();

        Self {
            header,
            widths,
            rows,
            sort: (sort_key, sort_direction),
        }
    }
}

impl<'a> StatefulWidget for FileInfoTable<'a> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = centered_rect(area, 60, 80);
        let block = Block::default().title("Files").borders(Borders::ALL);
        let table = Table::new(self.rows, self.widths)
            .block(block)
            .header(self.header)
            .highlight_spacing(HighlightSpacing::Always)
            .highlight_style(Style::default().bold().yellow().on_blue());

        StatefulWidget::render(table, area, buf, state);
    }
}
