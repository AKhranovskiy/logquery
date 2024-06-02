use std::{io::Stdout, path::Path};

use crossterm::event::{self};

use crate::{
    repository::Repository,
    utils::KeyEventExt,
    widgets::{FileList, FileListState, FileView, FileViewState, KeyEventHandler},
};

type Terminal = ratatui::Terminal<ratatui::backend::CrosstermBackend<Stdout>>;

pub struct App;

type Continue = bool;

impl App {
    pub fn run(terminal: &mut Terminal, target_dir: &Path) -> std::io::Result<()> {
        let mut state = AppState::new(target_dir);

        while Self::handle_key_events(&mut state)? {
            state.update();

            terminal.draw(|f| state.draw(f))?;
        }

        Ok(())
    }

    fn handle_key_events(state: &mut AppState) -> std::io::Result<Continue> {
        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key) = event::read()? {
                return Ok(state.handle_key_event(&key));
            }
        }
        Ok(true)
    }
}

pub struct AppState {
    repo: Repository,
    file_list: Option<FileListState>,
    files: FileViewState,
}

impl AppState {
    fn new(target_dir: &Path) -> Self {
        Self {
            repo: Repository::new(target_dir.to_owned()),
            file_list: Option::default(),
            files: FileViewState::default(),
        }
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        frame.render_stateful_widget(FileView {}, frame.size(), &mut self.files);

        if let Some(state) = self.file_list.as_mut() {
            frame.render_stateful_widget(FileList {}, frame.size(), state);
        }
    }

    fn handle_key_event(&mut self, event: &event::KeyEvent) -> Continue {
        if event.has_pressed('q') {
            return false;
        }

        if event.has_pressed('o') && self.file_list.is_none() {
            self.file_list = FileListState::default().into();
        } else if (event::KeyEventKind::Press, event::KeyCode::Esc) == (event.kind, event.code)
            && self.file_list.is_some()
            && !self.files.is_empty()
        {
            self.file_list = None;
        }

        if let Some(state) = self.file_list.as_mut() {
            if let Some(info) = state.handle_key_event(event) {
                self.files.push(info);
                self.file_list = None;
            }
        } else {
            self.files.handle_key_event(event);
        }

        true
    }

    fn update(&mut self) {
        if self.file_list.is_none() && self.files.is_empty() {
            self.file_list = FileListState::default().into();
        }

        if let Some(state) = self.file_list.as_mut() {
            state.update(&self.repo);
        };

        self.files.update(&self.repo);

        // TODO Updated file is not rendered
    }
}
