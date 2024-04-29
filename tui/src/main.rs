use std::{
    env::{args, current_exe},
    ffi::OsStr,
    io::{stdout, Result},
    path::{Path, PathBuf},
};

use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::{CrosstermBackend, Terminal},
    widgets::TableState,
};

use monitor::Monitor;

mod repository;
mod utils;
mod widgets;

use repository::{FileInfoListExt, FileInfoSortKey, Repository, SortDirection};
use widgets::FileInfoTable;

fn main() -> Result<()> {
    let Some(target_dir) = args()
        .nth(1)
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .filter(|p| p.is_dir())
    else {
        eprintln!(
            "Usage: {} <target-dir>",
            current_exe()
                .ok()
                .as_deref()
                .and_then(Path::file_name)
                .and_then(OsStr::to_str)
                .unwrap_or("<app>")
        );
        return Ok(());
    };

    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let mut repo = Repository::new();
    let mut monitor = Monitor::create(&target_dir).unwrap();

    let mut file_list_state = TableState::default();

    file_list_state.select(0.into());

    let mut file_list_sort_key = FileInfoSortKey::LastUpdate;
    let mut file_list_sort_direction = SortDirection::Descending;

    loop {
        while let Some(event) = monitor.try_next_event() {
            repo.update(event);
        }

        let file_info_list = repo
            .list()
            .sort(file_list_sort_key, file_list_sort_direction);

        let file_list_widget = FileInfoTable::new(
            &file_info_list,
            file_list_sort_key,
            file_list_sort_direction,
        );

        terminal.draw(|frame| {
            frame.render_stateful_widget(file_list_widget, frame.size(), &mut file_list_state);
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key) = event::read()? {
                match (key.kind, key.code) {
                    (KeyEventKind::Press, KeyCode::Char('q')) => break,

                    // File list table sorting
                    (KeyEventKind::Press, KeyCode::Char('n')) => {
                        file_list_sort_key = FileInfoSortKey::Name;
                        file_list_sort_direction = SortDirection::Ascending;
                    }
                    (KeyEventKind::Press, KeyCode::Char('N')) => {
                        file_list_sort_key = FileInfoSortKey::Name;
                        file_list_sort_direction = SortDirection::Descending;
                    }
                    (KeyEventKind::Press, KeyCode::Char('l')) => {
                        file_list_sort_key = FileInfoSortKey::LineCount;
                        file_list_sort_direction = SortDirection::Ascending;
                    }
                    (KeyEventKind::Press, KeyCode::Char('L')) => {
                        file_list_sort_key = FileInfoSortKey::LineCount;
                        file_list_sort_direction = SortDirection::Descending;
                    }
                    (KeyEventKind::Press, KeyCode::Char('a')) => {
                        file_list_sort_key = FileInfoSortKey::LastUpdate;
                        file_list_sort_direction = SortDirection::Ascending;
                    }
                    (KeyEventKind::Press, KeyCode::Char('A')) => {
                        file_list_sort_key = FileInfoSortKey::LastUpdate;
                        file_list_sort_direction = SortDirection::Descending;
                    }

                    // File list selection
                    (KeyEventKind::Press, KeyCode::Up) => {
                        file_list_state
                            .select(file_list_state.selected().map(|v| v.saturating_sub(1)));
                    }
                    (KeyEventKind::Press, KeyCode::Down) => {
                        file_list_state.select(
                            file_list_state
                                .selected()
                                .map(|v| v.saturating_add(1).min(file_info_list.len() - 1)),
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}
