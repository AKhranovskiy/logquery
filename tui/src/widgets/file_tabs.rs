#![allow(dead_code)]

use ratatui::{
    prelude::{Buffer, Rect},
    widgets::StatefulWidget,
};

pub type TabTitle = Box<str>;
pub type Tabs = Box<[TabTitle]>;

pub struct FileTabsState {
    tabs: Tabs,
    selected: usize,
}

impl FileTabsState {
    pub const fn new(tabs: Tabs) -> Self {
        assert!(!tabs.is_empty());
        Self { tabs, selected: 0 }
    }

    pub const fn selected(&self) -> usize {
        self.selected
    }

    pub fn select(&mut self, selected: usize) {
        assert!(selected < self.tabs.len());
        self.selected = selected;
    }
}

pub struct FileTabs;

impl StatefulWidget for FileTabs {
    type State = FileTabsState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let _ = (area, buf, state);
    }
}
