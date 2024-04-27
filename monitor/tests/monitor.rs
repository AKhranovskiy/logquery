use std::io::Write;

#[test]
pub fn test_monitor() {
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
