use std::io::Write;

use monitor::EventKind;

#[test]
pub fn test_monitor_new_files() {
    let temp_dir = tempfile::tempdir().unwrap();

    let mut m = monitor::Monitor::create(&temp_dir).unwrap();

    let mut temp_file_a = tempfile::NamedTempFile::new_in(&temp_dir).unwrap();

    temp_file_a.write_all(b"First line\n").unwrap();

    std::thread::sleep(std::time::Duration::from_millis(100));
    temp_file_a.write_all(b"Second line\nThird line").unwrap();

    std::thread::sleep(std::time::Duration::from_millis(100));
    temp_file_a.write_all(b"Ghost line").unwrap();
    drop(temp_file_a);

    std::thread::sleep(std::time::Duration::from_millis(100));

    assert_eq!(m.try_next_message().unwrap().kind, EventKind::Created);
    assert_eq!(m.try_next_message().unwrap().kind, EventKind::Modified);
    assert_eq!(m.try_next_message().unwrap().kind, EventKind::Modified);
    assert_eq!(m.try_next_message().unwrap().kind, EventKind::Modified);
    assert_eq!(m.try_next_message().unwrap().kind, EventKind::Removed);
}

#[test]
pub fn test_monitor_existing_files() {
    let temp_dir = tempfile::tempdir().unwrap();

    let mut file_a = tempfile::NamedTempFile::new_in(&temp_dir).unwrap();
    file_a.write_all(b"Line A\n").unwrap();

    let mut file_b = tempfile::NamedTempFile::new_in(&temp_dir).unwrap();
    file_b.write_all(b"Line C\n").unwrap();

    let mut m = monitor::Monitor::create(&temp_dir).unwrap();

    file_a.write_all(b"Line B\n").unwrap();
    file_b.write_all(b"Line D\n").unwrap();

    let events = (0..)
        .filter_map(|_| m.try_next_message())
        .map(|ev| ev.kind)
        .take(2)
        .collect::<Vec<_>>();

    assert_eq!(events, [EventKind::Modified, EventKind::Modified],);
}
