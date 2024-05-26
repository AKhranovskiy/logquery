use std::{io::Write, ops::RangeBounds};

use tempfile::NamedTempFile;

use line_index_reader::LineIndexReader;

#[rstest::rstest]
#[case::empty(empty(), 0)]
#[case::one_line_with_eof(one_line_eol(), 1)]
#[case::one_line_no_eof(one_line(), 1)]
#[case::small_no_eof(small_file(), SMALL_FILE_LINES)]
#[case::small_with_eof(small_file_eol(), SMALL_FILE_LINES)]
#[case::large(large_with_eof(), LARGE_FILE_LINES)]
#[tokio::test]
pub async fn index(#[case] file: NamedTempFile, #[case] expected_len: u32) {
    let index = LineIndexReader::index(&file).await.expect("LineIndex");

    assert_eq!(index.len(), expected_len);
}

#[rstest::rstest]
#[case::first(0, "Line 000000".into())]
#[case::middle(SMALL_FILE_LINES / 2, "Line 004782".into())]
#[case::last(SMALL_FILE_LINES - 1, "Line 009564".into())]
#[case::eof(SMALL_FILE_LINES, None)]
#[case::last_plus_one(SMALL_FILE_LINES + 1, None)]
#[case::beyond_eof(SMALL_FILE_LINES + 10, None)]
#[tokio::test]
pub async fn read_single_line(#[case] line: u32, #[case] expected: Option<&'static str>) {
    let file = small_file();
    let index = LineIndexReader::index(&file).await.expect("LineIndex");

    assert_eq!(expected, index.line(line).await.as_deref());
}

#[rstest::rstest]
#[case::from_start(..10, 11 * 10)]
#[case::beginning(0..10, 11 * 10)]
#[case::middle(SMALL_FILE_LINES / 3..SMALL_FILE_LINES / 2, 11 * 1_594)]
#[case::end(SMALL_FILE_LINES - 10..SMALL_FILE_LINES, 11 * 10)]
#[case::eof(SMALL_FILE_LINES - 10.., 11 * 10)]
#[case::beyond_eof(SMALL_FILE_LINES.., 0)]
#[case::all(.., 11 * SMALL_FILE_LINES as usize)]
#[tokio::test]
pub async fn read_many_lines<R>(#[case] lines: R, #[case] expected_size: usize)
where
    R: RangeBounds<u32> + Send,
{
    let file = small_file_eol();
    let index = LineIndexReader::index(&file).await.expect("LineIndex");
    let lines = index.lines(lines).await;

    assert_eq!(
        lines.iter().map(AsRef::as_ref).map(str::len).sum::<usize>(),
        expected_size
    );
}

#[rstest::rstest]
#[case::no_lines(0)]
#[case::one_line(1)]
#[case::many_lines(9)]
#[tokio::test]
pub async fn update(#[case] new_lines: u32) {
    let mut file = one_line();

    let index = LineIndexReader::index(&file).await.expect("LineIndex");
    assert_eq!(1, index.len());

    {
        for i in 1..=new_lines {
            write!(file, "\nLine {i:06}").unwrap();
        }
        file.flush().unwrap();
    }

    assert_eq!(new_lines, index.update().await.expect("Updated index"));
    assert_eq!(1 + new_lines, index.len());
}

#[rstest::rstest]
#[case::empty(empty())]
#[case::one(one_line())]
#[case::one_eol(one_line_eol())]
#[case::small(small_file())]
#[case::small_eol(small_file_eol())]
#[tokio::test]
pub async fn consistency(#[case] file: NamedTempFile) {
    let index = LineIndexReader::index(&file).await.expect("LineIndex");

    assert!(index
        .consistency()
        .await
        .expect("Index consistency")
        .is_consistent());
}

#[tokio::test]
pub async fn consistency_on_truncated() {
    let mut file = temp_file(10);
    let index = LineIndexReader::index(&file).await.expect("LineIndex");

    file.as_file_mut().set_len(11 * 5).expect("Truncated file");

    assert_eq!(
        5,
        index
            .consistency()
            .await
            .expect("Index consistency")
            .into_inconsistent()
            .expect("Inconsistent index")
    );
}

#[tokio::test]
pub async fn consistency_on_appended() {
    let mut file = temp_file(10);
    let index = LineIndexReader::index(&file).await.expect("LineIndex");

    for i in 10..15 {
        writeln!(file, "Line {i:06}").unwrap();
    }

    file.flush().unwrap();

    assert!(index
        .consistency()
        .await
        .expect("Index consistency")
        .is_consistent());
}

// 11 bytes per line, so under 100K lines
const SMALL_FILE_LINES: u32 = 9_565;
// 11 bytes per line, so over 100K lines
const LARGE_FILE_LINES: u32 = 123_456;

fn temp_file(lines: u32) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    for i in 0..lines {
        writeln!(f, "Line {i:06}").unwrap();
    }

    f.flush().unwrap();
    f
}

#[rstest::fixture]
#[once]
fn empty() -> NamedTempFile {
    NamedTempFile::new().unwrap()
}

#[rstest::fixture]
#[once]
fn one_line_eol() -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "Line 000000").unwrap();
    f.flush().unwrap();
    f
}

#[rstest::fixture]
#[once]
fn one_line() -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "Line 000000").unwrap();
    f.flush().unwrap();
    f
}

#[rstest::fixture]
#[once]
fn small_file_eol() -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    for i in 0..SMALL_FILE_LINES {
        writeln!(f, "Line {i:06}").unwrap();
    }
    f.flush().unwrap();
    f
}

#[rstest::fixture]
#[once]
fn small_file() -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    for i in 0..SMALL_FILE_LINES - 1 {
        writeln!(f, "Line {i:06}").unwrap();
    }
    write!(f, "Line {:06}", SMALL_FILE_LINES - 1).unwrap();
    f.flush().unwrap();
    f
}

#[rstest::fixture]
#[once]
fn large_with_eof() -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    for i in 0..LARGE_FILE_LINES {
        writeln!(f, "Line {i:06}").unwrap();
    }
    f.flush().unwrap();
    f
}
