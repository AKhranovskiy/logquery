use std::default;

use crossterm::event;

use crate::{
    repository::FileInfo,
    widgets::{FileList, FileListState, FileView, FileViewState, KeyEventHandler},
};

#[derive(enum_as_inner::EnumAsInner)]
pub enum ActiveWidget {
    FileList(FileListState),
    FileView(FileViewState),
}

impl default::Default for ActiveWidget {
    fn default() -> Self {
        Self::FileList(FileListState::default())
    }
}

impl ActiveWidget {
    pub fn draw(&mut self, frame: &mut ratatui::Frame) {
        if let Self::FileList(ref mut state) = self {
            frame.render_stateful_widget(FileList {}, frame.size(), state);
        } else if let Self::FileView(ref mut state) = self {
            frame.render_stateful_widget(FileView {}, frame.size(), state);
        }
    }
    // todo wrap result into enum
    pub fn handle_key_event(&mut self, event: &event::KeyEvent) -> Option<FileInfo> {
        match self {
            Self::FileList(state) => state.handle_key_event(event),
            Self::FileView(state) => {
                state.handle_key_event(event);
                None
            }
        }
    }

    pub fn file_list() -> Self {
        Self::FileList(FileListState::default())
    }

    pub fn file_view(info: FileInfo) -> Self {
        Self::FileView(info.into())
    }
}
