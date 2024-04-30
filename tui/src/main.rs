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
    widgets::Paragraph,
};

use monitor::Monitor;

mod repository;
mod utils;
mod widgets;

use repository::Repository;
use utils::centered_rect;
use widgets::{FileBrowser, FileBrowserState};

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

    let mut file_browser_state = FileBrowserState::new();

    loop {
        while let Some(event) = monitor.try_next_message() {
            repo.update(event);
        }
        file_browser_state.update(repo.list());

        terminal.draw(|frame| {
            if file_browser_state.is_visible() {
                frame.render_stateful_widget(FileBrowser {}, frame.size(), &mut file_browser_state);
            } else {
                frame.render_widget(
                    Paragraph::new("Press 'o' to open the file browser"),
                    centered_rect(frame.size(), 10, 10),
                );
            }
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key) = event::read()? {
                // TODO how to pass hanbdling result to file view?
                file_browser_state.handle_key_event(&key);

                if (key.kind, key.code) == (KeyEventKind::Press, KeyCode::Char('q')) {
                    break;
                }
            }
        }
    }

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}
