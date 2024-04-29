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
use ratatui::prelude::{CrosstermBackend, Terminal};

use monitor::Monitor;

mod repository;
mod utils;
mod widgets;

use repository::{FileInfoListExt, Repository, SortDirection};
use widgets::{FileInfoTable, FileInfoTableState};

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

    let mut file_list_state = FileInfoTableState::default();

    loop {
        while let Some(event) = monitor.try_next_event() {
            repo.update(event);
        }

        let file_info_list = repo.list().sort(
            repository::FileInfoSortKey::LastUpdate,
            SortDirection::Descending,
        );

        let file_list_widget = FileInfoTable::new(&file_info_list);

        terminal.draw(|frame| {
            frame.render_stateful_widget(file_list_widget, frame.size(), &mut file_list_state);
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}
