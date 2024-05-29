use std::default;

use crate::{
    repository::{FileInfo, RepoLines, RepoList},
    widgets::{FileList, FileListState, FileView, FileViewState},
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

    pub fn update(&mut self, list: &impl RepoList, lines: &impl RepoLines) {
        match self {
            Self::FileList(ref mut state) => {
                state.update(list);
            }
            Self::FileView(ref mut state) => {
                state.update(lines);
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
