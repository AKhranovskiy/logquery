use std::{io::Stdout, path::Path};

use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::widgets::Paragraph;

use crate::{
    repository::Repository,
    utils::centered_rect,
    widgets::{FileList, FileListAction, FileListState},
};

type Terminal = ratatui::Terminal<ratatui::backend::CrosstermBackend<Stdout>>;

pub struct App;

impl App {
    pub fn run(terminal: &mut Terminal, target_dir: &Path) -> std::io::Result<()> {
        let mut state = AppState::new(target_dir);

        while !state.quit {
            terminal.draw(|f| {
                state.active_widget.draw(f);
            })?;
            Self::handle_key_events(&mut state)?;
            state.update();
        }

        Ok(())
    }

    fn handle_key_events(state: &mut AppState) -> std::io::Result<()> {
        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key) = event::read()? {
                state.handle_key_event(&key);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, enum_as_inner::EnumAsInner)]
enum ActiveWidget {
    FileList(FileListState),
    Test(String),
}

impl ActiveWidget {
    fn draw(&mut self, frame: &mut ratatui::Frame) {
        if let Self::FileList(ref mut state) = self {
            frame.render_stateful_widget(FileList {}, frame.size(), state);
        } else if let Self::Test(ref text) = self {
            frame.render_widget(
                Paragraph::new(vec![
                    text.as_str().into(),
                    "Press 'o' to open the file browser".into(),
                ]),
                centered_rect(frame.size(), 30, 10),
            );
        }
    }
}
struct AppState {
    quit: bool,
    active_widget: ActiveWidget,
    repo: Repository,
}

impl AppState {
    fn new(target_dir: &Path) -> Self {
        Self {
            quit: false,
            active_widget: ActiveWidget::FileList(FileListState::new()),
            repo: Repository::new(target_dir),
        }
    }

    fn handle_key_event(&mut self, key: &event::KeyEvent) {
        if (key.kind, key.code) == (KeyEventKind::Press, KeyCode::Char('q')) {
            self.quit = true;
        } else if let ActiveWidget::FileList(ref mut state) = &mut self.active_widget {
            match state.handle_key_event(key) {
                FileListAction::None => {}
                FileListAction::OpenNewTab(info) => {
                    self.active_widget =
                        ActiveWidget::Test(format!("Open in the new tab: {}", info.name));
                }
                FileListAction::SplitCurrentTab(info) => {
                    self.active_widget =
                        ActiveWidget::Test(format!("Split the current tab: {}", info.name));
                }
                FileListAction::MergeIntoCurrentTab(info) => {
                    self.active_widget =
                        ActiveWidget::Test(format!("Merge into the current tab: {}", info.name));
                }
            }
        } else if (key.kind, key.code) == (KeyEventKind::Press, KeyCode::Char('o')) {
            self.active_widget = ActiveWidget::FileList(FileListState::new());
        }
    }

    fn update(&mut self) {
        if let ActiveWidget::FileList(ref mut state) = &mut self.active_widget {
            state.update(self.repo.list());
        }
    }
}
