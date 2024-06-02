use std::{path::PathBuf, sync::Arc};

use dashmap::{mapref::multiple::RefMulti, DashMap};
use itertools::Itertools;
use time::OffsetDateTime;
use tokio::sync::{
    mpsc,
    oneshot::{self},
};

use line_cache::LineCache;
use line_index_reader::LineIndexReader;
use monitor::Monitor;

use crate::utils::{self, file_name};

struct Entry {
    reader: Arc<LineIndexReader>,
    line_cache: Arc<LineCache>,
    updated: OffsetDateTime,
}

impl From<LineIndexReader> for Entry {
    fn from(reader: LineIndexReader) -> Self {
        let reader = Arc::new(reader);
        let line_cache = Arc::new(LineCache::new(reader.clone()));
        Self {
            reader,
            line_cache,
            updated: utils::now(),
        }
    }
}

type LinesRequest = (Arc<LineCache>, u32, u32);

pub struct Repository {
    entries: Arc<DashMap<String, Entry>>,
    lines_sender: mpsc::Sender<LinesRequest>,
    #[allow(dead_code)]
    watcher: oneshot::Sender<()>,
}

impl Repository {
    pub fn new(target_dir: PathBuf) -> Self {
        let entries = Arc::new(DashMap::new());
        let entries_clone = entries.clone();

        let (watcher, is_dead) = oneshot::channel::<()>();
        let (lines_request_sender, lines_request_receiver) = mpsc::channel::<LinesRequest>(1024);

        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .build()
                .unwrap()
                .block_on(async move {
                    Self::worker(target_dir, is_dead, entries_clone, lines_request_receiver).await;
                });
        });

        Self {
            entries,
            lines_sender: lines_request_sender,
            watcher,
        }
    }

    async fn worker(
        target_dir: PathBuf,
        mut is_dead: oneshot::Receiver<()>,
        file_entries: Arc<DashMap<String, Entry>>,
        mut lines_request: mpsc::Receiver<LinesRequest>,
    ) {
        let mut monitor = Monitor::create(&target_dir).unwrap();

        loop {
            tokio::select! {
                    _ = &mut is_dead => {
                        break;
                    }
                    Some(event) = monitor.next_message() => {
                        Self::handle_event(event, &file_entries).await;
                    }
                    Some((line_cache, from, to)) = lines_request.recv() => {
                        line_cache.lines(from..to).await;
                    }
            }
        }
    }

    async fn handle_event(event: monitor::Event, entries: &Arc<DashMap<String, Entry>>) {
        let Some(name) = file_name(&event.path) else {
            return;
        };

        match event.kind {
            monitor::EventKind::Created => {
                if let Ok(reader) = LineIndexReader::index(&event.path).await {
                    entries.insert(name, reader.into());
                };
            }
            monitor::EventKind::Modified => {
                if let Some(mut entry) = entries.get_mut(&name) {
                    if entry.reader.update().await.is_ok() {
                        entry.updated = utils::now();
                    }
                }
            }
            monitor::EventKind::Removed => {
                entries.remove(&name);
            }
        }
    }
}

pub trait RepoList {
    fn list(&self) -> Vec<FileInfo>;
}

impl RepoList for Repository {
    fn list(&self) -> Vec<FileInfo> {
        self.entries.iter().map(Into::into).collect()
    }
}

pub trait RepoLines {
    fn lines(&self, name: &str, from: u32, to: u32) -> Box<[Arc<str>]>;
    fn total(&self, name: &str) -> u32;
}

impl RepoLines for Repository {
    fn lines(&self, name: &str, from: u32, to: u32) -> Box<[Arc<str>]> {
        let Some(entry) = self.entries.get(name) else {
            return Box::default();
        };

        let lines = entry.value().line_cache.lines_opt(from..to);

        if lines.iter().any(Option::is_none) {
            self.lines_sender
                .try_send((entry.value().line_cache.clone(), from, to))
                .unwrap();
        }

        lines
            .iter()
            .map_while(Clone::clone)
            .collect_vec()
            .into_boxed_slice()
    }

    fn total(&self, name: &str) -> u32 {
        self.entries
            .get(name)
            .map(|entry| entry.value().reader.len())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileInfo {
    pub name: String,
    pub last_update: OffsetDateTime,
    pub number_of_lines: u32,
}

impl From<RefMulti<'_, String, Entry>> for FileInfo {
    fn from(entry: RefMulti<String, Entry>) -> Self {
        Self {
            name: entry.key().clone(),
            last_update: entry.value().updated,
            number_of_lines: entry.value().reader.len(),
        }
    }
}
