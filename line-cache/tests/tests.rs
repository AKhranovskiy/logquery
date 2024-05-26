use std::{io::Write, sync::Arc};

use line_cache::LineCache;
use line_index_reader::LineIndexReader;

#[tokio::test]
async fn test_empty_file() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let reader = Arc::new(LineIndexReader::index(file.path()).await.unwrap());
    let cache = LineCache::new(reader);

    assert!(cache.lines(..).await.is_empty());
    assert!(cache.line(0).await.is_none());
}

#[tokio::test]
async fn test_non_empty_file() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    for i in 0..10 {
        file.write_all(format!("Line {i:03}\n").as_bytes()).unwrap();
    }
    file.flush().unwrap();

    let reader = Arc::new(LineIndexReader::index(file.path()).await.unwrap());
    assert_eq!(reader.len(), 10);

    let cache = LineCache::new(reader);

    assert_eq!(cache.lines(..).await.len(), 10);
    assert_eq!(
        cache
            .line(0)
            .await
            .expect("should return first line")
            .as_ref(),
        "Line 000"
    );
    assert_eq!(
        cache
            .line(9)
            .await
            .expect("should return last line")
            .as_ref(),
        "Line 009"
    );
    assert!(cache.line(10).await.is_none());
}
