use std::{
    env::{args, current_exe},
    ffi::OsStr,
    io::{stdout, Result, Stdout},
    path::{Path, PathBuf},
};

use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::{CrosstermBackend, Terminal};
use tracing_subscriber::util::SubscriberInitExt;

mod active_widget;
mod app;
mod repository;
mod utils;
mod widgets;

use crate::app::App;

fn main() -> Result<()> {
    let Some(target_dir) = target_dir_from_args() else {
        print_usage();
        return Ok(());
    };

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::DEBUG)
        .compact()
        .finish()
        .init();

    with_terminal(|terminal| App::run(terminal, &target_dir))
}

fn with_terminal<F>(f: F) -> Result<()>
where
    F: FnOnce(&mut Terminal<CrosstermBackend<Stdout>>) -> Result<()>,
{
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let result = f(&mut terminal);

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    result
}

fn target_dir_from_args() -> Option<PathBuf> {
    args()
        .nth(1)
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .filter(|p| p.is_dir())
}

fn print_usage() {
    eprintln!(
        "Usage: {} <target-dir>",
        current_exe()
            .ok()
            .as_deref()
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
            .unwrap_or("<app>")
    );
}
