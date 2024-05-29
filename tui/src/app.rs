use std::{io::Stdout, path::Path};

use crossterm::event::{self, KeyCode, KeyEventKind};

use crate::{active_widget::ActiveWidget, repository::Repository, widgets::KeyEventHandler};

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

pub struct AppState {
    quit: bool,
    active_widget: ActiveWidget,
    pub repo: Repository,
}

impl AppState {
    fn new(target_dir: &Path) -> Self {
        Self {
            quit: false,
            active_widget: ActiveWidget::default(),
            repo: Repository::new(target_dir.to_owned()),
        }
    }

    fn handle_key_event(&mut self, key: &event::KeyEvent) {
        if (key.kind, key.code) == (KeyEventKind::Press, KeyCode::Char('q')) {
            self.quit = true;
        } else if let ActiveWidget::FileList(ref mut state) = &mut self.active_widget {
            if let Some(info) = state.handle_key_event(key) {
                self.active_widget = ActiveWidget::file_view(info);
            }
        } else if (key.kind, key.code) == (KeyEventKind::Press, KeyCode::Char('o')) {
            self.active_widget = ActiveWidget::file_list();
        } else if let ActiveWidget::FileView(ref mut state) = &mut self.active_widget {
            state.handle_key_event(key);
        }
    }

    fn update(&mut self) {
        self.active_widget.update(&self.repo, &self.repo);
    }
}
