use std::path::{Path, PathBuf};

use enum_as_inner::EnumAsInner;
use notify::Watcher;
use tap::TapFallible;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Send error: {0}")]
    SendFailure(#[from] tokio::sync::mpsc::error::SendError<Event>),
    #[error("Notify error: {0}")]
    NotifyError(#[from] notify::Error),
}

pub struct Monitor {
    #[allow(dead_code)]
    watcher: notify::RecommendedWatcher,
    events: UnboundedReceiver<Event>,
}

impl Monitor {
    pub fn create<P>(path: &P) -> Result<Self, Error>
    where
        P: AsRef<Path> + Send,
    {
        // TODO bound
        let (tx, rx) = unbounded_channel();

        for event in list_files_in_directory(path)? {
            tx.send(event).tap_err(|error| {
                tracing::error!(path = %path.as_ref().display(), %error, "Failed to send initial list of files");
            })?;
        }

        let mut watcher = notify::recommended_watcher({
            move |res: notify::Result<notify::Event>| {
                let event = res.expect("Notify event");
                for ev in event
                    .paths
                    .iter()
                    .filter_map(|path| event_handler(path.to_owned(), event.kind))
                {
                    let path = ev.path.clone();
                    _ = tx.send(ev).tap_err(|error| {
                        tracing::error!(path = %path.display(), event_kind = ?event.kind, %error, "Failed to send an event");
                    });
                }
            }
        })?;
        watcher.watch(path.as_ref(), notify::RecursiveMode::NonRecursive)?;

        Ok(Self {
            watcher,
            events: rx,
        })
    }

    pub fn try_next_message(&mut self) -> Option<Event> {
        self.events.try_recv().ok()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumAsInner)]
pub enum EventKind {
    Created,
    Modified,
    Removed,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub path: PathBuf,
    pub kind: EventKind,
}

fn event_handler(path: PathBuf, event_kind: notify::EventKind) -> Option<Event> {
    match event_kind {
        notify::EventKind::Access(_) => None, /* Access events are ignored */
        notify::EventKind::Create(notify::event::CreateKind::File) => Event {
            path,
            kind: EventKind::Created,
        }
        .into(),
        notify::EventKind::Modify(_) => Event {
            path,
            kind: EventKind::Modified,
        }
        .into(),
        notify::EventKind::Remove(notify::event::RemoveKind::File) => Event {
            path,
            kind: EventKind::Removed,
        }
        .into(),
        kind => {
            tracing::warn!("Unsupported event {kind:?} for file {}", path.display());
            None
        }
    }
}

fn list_files_in_directory<P>(path: &P) -> Result<Vec<Event>, Error>
where
    P: AsRef<Path>,
{
    std::fs::read_dir(path)
        .map(|res| {
            res.map(|entry| entry.map(|entry| entry.path()))
                .filter_map(Result::ok)
                .filter(|path| path.is_file())
                .filter(|path| path.extension() == Some("log".as_ref()))
                .map(|path| Event {
                    path,
                    kind: EventKind::Created,
                })
                .collect()
        })
        .map_err(Into::into)
}
