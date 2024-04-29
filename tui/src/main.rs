use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

fn main() {
    let Some(target_dir) = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .filter(|p| p.is_dir())
    else {
        eprintln!(
            "Usage: {} <target-dir>",
            std::env::current_exe()
                .ok()
                .as_deref()
                .and_then(Path::file_name)
                .and_then(OsStr::to_str)
                .unwrap_or("<app>")
        );
        return;
    };

    futures::executor::block_on(async {
        let mut monitor = monitor::Monitor::create(&target_dir).unwrap();
        loop {
            let event = monitor.wait_for_next_event().await;
            let now = time::OffsetDateTime::now_utc()
                .checked_to_offset(time::macros::offset!(+7))
                .unwrap();
            match event.kind {
                monitor::EventKind::Created => {
                    println!("{now}\t{}\tCREATED", event.path.display());
                }
                monitor::EventKind::NewLine(line) => {
                    println!(
                        "{now}\t{}\t{}",
                        event.path.display(),
                        line.get(..30).unwrap_or_default()
                    );
                }
                monitor::EventKind::Removed => {
                    println!("{now}\t{}\tREMOVED", event.path.display());
                }
            }
        }
    });
}
