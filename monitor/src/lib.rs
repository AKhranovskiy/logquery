use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

use futures::{channel::mpsc::UnboundedReceiver, lock::Mutex, StreamExt};
use notify::Watcher;

pub struct Monitor {
    #[allow(dead_code)]
    watcher: notify::RecommendedWatcher,
    events: UnboundedReceiver<Event>,
}

impl Monitor {
    pub fn create<P>(path: &P) -> Result<Self, Box<dyn std::error::Error>>
    where
        P: AsRef<Path> + Send,
    {
        // TODO bound
        let (tx, rx) = futures::channel::mpsc::unbounded();

        // Note: must be declared outsize of the handler block.
        let readers: Arc<Mutex<HashMap<PathBuf, File>>> = <_>::default();

        let handler = move |res: Result<notify::Event, notify::Error>| {
            futures::executor::block_on(async {
                match res {
                    Ok(ev) => match ev.kind {
                        notify::EventKind::Access(_) => { /* Access events are ignored */ }
                        notify::EventKind::Create(notify::event::CreateKind::File) => {
                            for path in ev.paths {
                                assert!(path.exists());

                                readers
                                    .lock()
                                    .await
                                    .insert(path.clone(), File::open(&path).unwrap());

                                tx.unbounded_send(Event {
                                    path,
                                    kind: EventKind::Created,
                                })
                                .unwrap();
                            }
                        }
                        notify::EventKind::Modify(_) => {
                            for path in ev.paths {
                                if !path.exists() {
                                    // Likely the file has been removed right after update, which may happen with temp files.
                                    // There must be Removed event from notify, so nothing to do here.
                                    continue;
                                }

                                let mut buf = vec![];

                                readers
                                    .lock()
                                    .await
                                    .entry(path.clone())
                                    .or_insert_with(|| File::open(&path).unwrap())
                                    .read_to_end(&mut buf)
                                    .unwrap();

                                let lines = buf.lines().collect::<Result<Vec<_>, _>>().unwrap();

                                for line in lines {
                                    tx.unbounded_send(Event {
                                        path: path.clone(),
                                        kind: EventKind::NewLine(line),
                                    })
                                    .unwrap();
                                }
                            }
                        }
                        notify::EventKind::Remove(notify::event::RemoveKind::File) => {
                            for path in ev.paths {
                                assert!(!path.exists());

                                readers.lock().await.remove(&path);

                                tx.unbounded_send(Event {
                                    path,
                                    kind: EventKind::Removed,
                                })
                                .unwrap();
                            }
                        }
                        kind => todo!("{:?}", kind),
                    },
                    Err(error) => panic!("watch failed: {error}"),
                }
            });
        };

        let mut watcher = notify::recommended_watcher(handler)?;
        watcher.watch(path.as_ref(), notify::RecursiveMode::NonRecursive)?;

        Ok(Self {
            watcher,
            events: rx,
        })
    }

    /// Wait for the next event and return it.
    pub async fn wait_for_next_event(&mut self) -> Event {
        self.events.select_next_some().await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    Created,
    NewLine(String),
    Removed,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub path: PathBuf,
    pub kind: EventKind,
}
