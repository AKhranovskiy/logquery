use std::{path::Path, sync::Arc};

use dashmap::{mapref::multiple::RefMulti, DashMap};
use time::OffsetDateTime;
use tokio::sync::oneshot::{channel, error::TryRecvError, Sender};

use line_index_reader::LineIndexReader;
use monitor::Monitor;

use crate::utils::{self, file_name};

struct Entry {
    reader: LineIndexReader,
    updated: OffsetDateTime,
}

pub struct Repository {
    entires: Arc<DashMap<String, Entry>>,
    #[allow(dead_code)]
    worker: (Sender<()>, std::thread::JoinHandle<()>),
}

impl Repository {
    pub fn new(target_dir: &Path) -> Self {
        let entires = Arc::new(DashMap::new());
        let entries_clone = Arc::clone(&entires);

        let (sender, mut receiver) = channel::<()>();

        let target_dir = target_dir.to_owned();
        let mut monitor = Monitor::create(&target_dir).unwrap();

        let thread_handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            loop {
                match receiver.try_recv() {
                    Ok(()) | Err(TryRecvError::Closed) => {
                        break;
                    }
                    Err(TryRecvError::Empty) => {}
                }

                while let Some(event) = monitor.try_next_message() {
                    let Some(name) = file_name(&event.path) else {
                        continue;
                    };

                    match event.kind {
                        monitor::EventKind::Created => {
                            if let Ok(reader) = rt.block_on(LineIndexReader::index(&event.path)) {
                                entries_clone.insert(
                                    name,
                                    Entry {
                                        reader,
                                        updated: utils::now(),
                                    },
                                );
                            }
                        }
                        monitor::EventKind::Modified => {
                            if let Some(mut entry) = entries_clone.get_mut(&name) {
                                if rt.block_on(entry.reader.update()).is_ok() {
                                    entry.updated = utils::now();
                                }
                            }
                        }
                        monitor::EventKind::Removed => {
                            entries_clone.remove(&name);
                        }
                    }
                }
            }
        });

        Self {
            entires,
            worker: (sender, thread_handle),
        }
    }

    pub fn list(&self) -> Vec<FileInfo> {
        self.entires.iter().map(Into::into).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileInfo {
    pub name: String,
    pub last_update: OffsetDateTime,
    pub number_of_lines: usize,
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
