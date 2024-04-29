use itertools::Itertools;
use ratatui::{
    prelude::{Buffer, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, HighlightSpacing, List, ListItem, ListState, StatefulWidget, Widget,
    },
};
use time::macros::format_description;

use crate::{
    repository::FileInfo,
    utils::{self, centered_rect},
};

pub struct FileInfoTable<'a> {
    items: Vec<ListItem<'a>>,
}

impl<'a> FileInfoTable<'a> {
    pub fn new(files: &'a [FileInfo]) -> Self {
        let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
        let items = files
            .iter()
            .map(|file| {
                ListItem::new(Text::from(Line::from(vec![
                    Span::raw(file.name.as_str()),
                    Span::raw("  "),
                    Span::raw(file.number_of_lines.to_string()),
                    Span::raw("  "),
                    Span::raw(file.last_update.format(&format).unwrap_or_default()),
                    Span::raw(" / "),
                    Span::raw(
                        (utils::now() - file.last_update)
                            .whole_seconds()
                            .to_string(),
                    ),
                    Span::raw(" s"),
                ])))
            })
            .collect_vec();

        Self { items }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct FileInfoTableState {
    list_state: ListState,
}

impl<'a> StatefulWidget for FileInfoTable<'a> {
    type State = FileInfoTableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.list_state.select(0.into());

        let area = centered_rect(area, 60, 80);
        let block = Block::default().title("Files").borders(Borders::ALL);
        let list = List::new(self.items)
            .block(block)
            .highlight_spacing(HighlightSpacing::Always)
            .highlight_style(Style::default().bold().yellow().on_blue());

        StatefulWidget::render(list, area, buf, &mut state.list_state);
    }
}
