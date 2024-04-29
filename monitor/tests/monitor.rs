use std::io::Write;

#[test]
pub fn test_monitor_new_files() {
    let temp_dir = tempfile::tempdir().unwrap();

    let mut m = monitor::Monitor::create(&temp_dir).unwrap();

    let mut temp_file_a = tempfile::NamedTempFile::new_in(&temp_dir).unwrap();

    temp_file_a.write_all(b"First line\n").unwrap();
    temp_file_a.flush().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(100));
    temp_file_a.write_all(b"Second line\nThird line").unwrap();

    std::thread::sleep(std::time::Duration::from_millis(100));
    temp_file_a.write_all(b"Ghost line").unwrap();
    drop(temp_file_a);

    futures::executor::block_on(async {
        assert!(m.wait_for_next_event().await.kind == monitor::EventKind::Created);
        assert!(
            m.wait_for_next_event().await.kind
                == monitor::EventKind::NewLine("First line".to_string())
        );
        assert!(
            m.wait_for_next_event().await.kind
                == monitor::EventKind::NewLine("Second line".to_string())
        );
        assert!(
            m.wait_for_next_event().await.kind
                == monitor::EventKind::NewLine("Third line".to_string())
        );
        assert!(m.wait_for_next_event().await.kind == monitor::EventKind::Removed);
    });
}

#[test]
pub fn test_monitor_existing_files() {
    let temp_dir = tempfile::tempdir().unwrap();

    let mut file_a = tempfile::NamedTempFile::new_in(&temp_dir).unwrap();
    file_a.write_all(b"Line A\n").unwrap();
    file_a.write_all(b"Line B\n").unwrap();

    let mut file_b = tempfile::NamedTempFile::new_in(&temp_dir).unwrap();
    file_b.write_all(b"Line C\n").unwrap();
    file_b.write_all(b"Line D\n").unwrap();

    let mut m = monitor::Monitor::create(&temp_dir).unwrap();

    futures::executor::block_on(async {
        // Order of files are not guaranteed, but the order of lines is stable.
        let line_1 = m.wait_for_next_event().await.kind.into_new_line().unwrap();
        let line_2 = m.wait_for_next_event().await.kind.into_new_line().unwrap();
        let line_3 = m.wait_for_next_event().await.kind.into_new_line().unwrap();
        let line_4 = m.wait_for_next_event().await.kind.into_new_line().unwrap();

        if line_1 == "Line A" {
            assert_eq!(line_2, "Line B");
            assert_eq!(line_3, "Line C");
            assert_eq!(line_4, "Line D");
        } else if line_1 == "Line C" {
            assert_eq!(line_2, "Line D");
            assert_eq!(line_3, "Line A");
            assert_eq!(line_4, "Line B");
        } else {
            panic!("Unexpected lines: {line_1}, {line_2}, {line_3}, {line_4}");
        }
    });
}
