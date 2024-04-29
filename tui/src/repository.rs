use std::{borrow::Cow, collections::HashMap, ffi::OsStr, path::Path};

use itertools::Itertools;
use time::OffsetDateTime;

use crate::utils;

pub struct Repository {
    lines: HashMap<String, Vec<String>>,
    updates: HashMap<String, OffsetDateTime>,
}

impl Repository {
    pub fn new() -> Self {
        Self {
            lines: HashMap::new(),
            updates: HashMap::new(),
        }
    }

    pub fn update(&mut self, event: monitor::Event) {
        let name = file_name(&event.path);

        match event.kind {
            monitor::EventKind::Created => {
                if self.lines.insert(name.clone(), vec![]).is_some() {
                    eprintln!("Replace the file content: {}", event.path.display());
                }
                self.updates.insert(name, utils::now());
            }
            monitor::EventKind::NewLine(line) => {
                self.lines.entry(name.clone()).or_default().push(line);
                self.updates.insert(name, utils::now());
            }
            monitor::EventKind::Removed => {
                self.lines.remove(&name);
                self.updates.remove(&name);
            }
        }
    }

    pub fn list(&self) -> Vec<FileInfo> {
        self.lines
            .iter()
            .map(|(name, lines)| FileInfo {
                name: name.clone(),
                last_update: self.updates.get(name).copied().unwrap_or_else(utils::now),
                number_of_lines: lines.len(),
            })
            .collect_vec()
    }

    pub fn content(&self, file: &str) -> &[String] {
        static EMPTY: Vec<String> = vec![];
        self.lines.get(file).unwrap_or(&EMPTY)
    }
}

fn file_name(path: &Path) -> String {
    path.file_stem()
        .map(OsStr::to_string_lossy)
        .as_ref()
        .map_or_else(|| "UKNOWN".to_string(), Cow::to_string)
}

pub struct FileInfo {
    pub name: String,
    pub last_update: OffsetDateTime,
    pub number_of_lines: usize,
}

pub enum FileInfoSortKey {
    Name,
    LastUpdate,
    LineCount,
}

pub enum SortDirection {
    Ascending,
    Descending,
}

pub trait FileInfoListExt {
    fn sort(self, key: FileInfoSortKey, direction: SortDirection) -> Self;
}

impl FileInfoListExt for Vec<FileInfo> {
    fn sort(self, key: FileInfoSortKey, direction: SortDirection) -> Self {
        let cmp = match key {
            FileInfoSortKey::Name => |a: &FileInfo, b: &FileInfo| a.name.cmp(&b.name),
            FileInfoSortKey::LastUpdate => {
                |a: &FileInfo, b: &FileInfo| a.last_update.cmp(&b.last_update)
            }
            FileInfoSortKey::LineCount => {
                |a: &FileInfo, b: &FileInfo| a.number_of_lines.cmp(&b.number_of_lines)
            }
        };

        let sorted = self.into_iter().sorted_by(cmp);

        match direction {
            SortDirection::Ascending => sorted.collect(),
            SortDirection::Descending => sorted.rev().collect(),
        }
    }
}
