use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, Read},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use futures::{channel::mpsc::UnboundedReceiver, StreamExt};
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

        // Note: must live outside of the watcher.
        let files: Arc<Mutex<HashMap<PathBuf, File>>> = <_>::default();

        let handler = move |res: Result<notify::Event, notify::Error>| match res {
            Ok(ev) => {
                for path in &ev.paths {
                    let events = {
                        match event_handler(files.as_ref(), path, ev.kind) {
                            Ok(events) => events,
                            Err(error) => {
                                eprintln!(
                                    "Failed to handle {:?} event for {}: {error}",
                                    ev.kind,
                                    path.display()
                                );
                                vec![]
                            }
                        }
                    };

                    for event in events {
                        if let Err(error) = tx.unbounded_send(event.clone()) {
                            eprintln!("Failed to send {event:?}: {error}",);
                        }
                    }
                }
            }

            Err(error) => panic!("watch failed: {error}"),
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

fn event_handler(
    files: &Mutex<HashMap<PathBuf, File>>,
    path: &Path,
    event_kind: notify::EventKind,
) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
    match event_kind {
        notify::EventKind::Access(_) => Ok(vec![]), /* Access events are ignored */
        notify::EventKind::Create(notify::event::CreateKind::File) => {
            if !path.exists() {
                eprintln!(
                    "Received Create(File) event for non-existing file {}",
                    path.display()
                );
                return Ok(vec![]);
            }

            let Ok(file) = File::open(path) else {
                eprintln!("Failed to open file {}", path.display());
                return Ok(vec![]);
            };

            if files
                .lock()
                .unwrap()
                .insert(path.to_owned(), file)
                .is_some()
            {
                eprintln!("File {} was already opened", path.display());
            }

            Ok(vec![Event {
                path: path.to_owned(),
                kind: EventKind::Created,
            }])
        }
        notify::EventKind::Modify(_) => {
            if !path.exists() {
                eprintln!(
                    "Received Create(File) event for non-existing file {}",
                    path.display()
                );
                return Ok(vec![]);
            }

            let mut buf = vec![];

            files
                .lock()
                .unwrap()
                .entry(path.to_owned())
                .or_insert_with(|| File::open(path).unwrap())
                .read_to_end(&mut buf)?;

            Ok(buf
                .lines()
                .map_while(Result::ok)
                .map(|line| Event {
                    path: path.to_owned(),
                    kind: EventKind::NewLine(line),
                })
                .collect())
        }
        notify::EventKind::Remove(notify::event::RemoveKind::File) => {
            if path.exists() {
                eprintln!(
                    "Received Remove(File) event for existing file {}",
                    path.display()
                );
                return Ok(vec![]);
            }
            if files.lock().unwrap().remove(path).is_none() {
                eprintln!(
                    "Received Remove(File) event for non-monitored file {}",
                    path.display()
                );
            }

            Ok(vec![Event {
                path: path.to_owned(),
                kind: EventKind::Removed,
            }])
        }
        kind => Err(format!("Unsupported event kind {kind:?}").into()),
    }
}
