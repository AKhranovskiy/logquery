use std::{io::Stdout, path::Path};

use crossterm::event::{self};

use crate::{active_widget::ActiveWidget, repository::Repository, utils::KeyEventExt};

type Terminal = ratatui::Terminal<ratatui::backend::CrosstermBackend<Stdout>>;

pub struct App;

impl App {
    pub fn run(terminal: &mut Terminal, target_dir: &Path) -> std::io::Result<()> {
        let mut state = AppState::new(target_dir);

        while !state.quit {
            terminal.draw(|f| {
                state.draw(f);
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
    repo: Repository,
}

impl AppState {
    fn new(target_dir: &Path) -> Self {
        Self {
            quit: false,
            active_widget: ActiveWidget::default(),
            repo: Repository::new(target_dir.to_owned()),
        }
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        self.active_widget.draw(frame);
    }

    fn handle_key_event(&mut self, event: &event::KeyEvent) {
        if event.has_pressed('q') {
            self.quit = true;
        } else if event.has_pressed('o') && !self.active_widget.is_file_list() {
            self.active_widget = ActiveWidget::file_list();
        } else if let Some(info) = self.active_widget.handle_key_event(event) {
            self.active_widget = ActiveWidget::file_view(info);
        }
    }

    fn update(&mut self) {
        match self.active_widget {
            ActiveWidget::FileList(ref mut state) => {
                state.update(&self.repo);
            }
            ActiveWidget::FileView(ref mut state) => {
                state.update(&self.repo);
            }
        }
    }
}
