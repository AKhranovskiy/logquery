use std::{
    collections::HashMap,
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
use itertools::Itertools;
use monitor::{EventKind, Monitor};
use ratatui::{
    prelude::{CrosstermBackend, Stylize, Terminal},
    style::Style,
    widgets::{List, ListState},
};

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

    let mut monitor = Monitor::create(&target_dir).unwrap();
    let mut events: HashMap<String, Vec<EventKind>> = HashMap::new();
    // let mut updated_tabs: HashSet<&str> = HashSet::new();
    // let mut selected_tab: Option<&str> = None;

    loop {
        while let Some(event) = monitor.try_next_event() {
            let label = event
                .path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();
            events.entry(label).or_default().push(event.kind);
        }

        let labels = events.keys().map(String::as_str).sorted().collect_vec();

        let mut tab_state = ListState::default().with_selected(0.into());
        let tabs = List::new(labels)
            .highlight_spacing(ratatui::widgets::HighlightSpacing::Always)
            .highlight_style(Style::default().bold());
        // let mut lines = vec![];
        // for (now, event) in &events {
        //     match &event.kind {
        //         EventKind::Created => {
        //             lines.push(Line::from(vec![
        //                 Span::styled(now.to_string(), Style::default().bold()),
        //                 Span::raw("\t"),
        //                 Span::styled(event.path.display().to_string(), Style::default().bold()),
        //                 Span::raw("\t"),
        //                 Span::styled("CREATED", Style::default().green()),
        //             ]));
        //         }
        //         EventKind::NewLine(line) => {
        //             lines.push(Line::from(vec![
        //                 Span::styled(now.to_string(), Style::default().bold()),
        //                 Span::raw("\t"),
        //                 Span::styled(event.path.display().to_string(), Style::default().bold()),
        //                 Span::raw("\t"),
        //                 Span::raw(line.get(..45).unwrap_or_default()),
        //             ]));
        //         }
        //         EventKind::Removed => {
        //             lines.push(Line::from(vec![
        //                 Span::styled(now.to_string(), Style::default().bold()),
        //                 Span::raw("\t"),
        //                 Span::styled(event.path.display().to_string(), Style::default().bold()),
        //                 Span::raw("\t"),
        //                 Span::styled("REMOVED", Style::default().red()),
        //             ]));
        //         }
        //     }
        // }

        // let text = Text::from(lines);
        // let p = Paragraph::new(text).scroll((
        //     events
        //         .len()
        //         .saturating_sub(terminal.get_frame().size().height as usize) as u16,
        //     0,
        // ));

        terminal.draw(|frame| {
            let area = frame.size();
            frame.render_stateful_widget(tabs, area, &mut tab_state);
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
