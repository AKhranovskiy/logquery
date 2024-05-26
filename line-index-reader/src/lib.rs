use std::{
    io::{BufRead, Seek, SeekFrom},
    ops::{Bound, RangeBounds},
    path::{Path, PathBuf},
    sync::RwLock,
};

use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, BufReader},
    task::spawn_blocking,
};

const READ_BUF_CAPACITY: usize = 8_192;

pub type Line = Box<str>;
pub type Lines = Box<[Line]>;

pub struct LineIndexReader {
    path: PathBuf,
    offsets: RwLock<Vec<u64>>,
}

/// Common interface
impl LineIndexReader {
    pub async fn index<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path> + Clone + Send,
    {
        let file = File::open(path.clone()).await?;
        let offsets = spawn_blocking(move || index_lines(file)).await.unwrap()?;

        Ok(Self {
            path: path.as_ref().to_owned(),
            offsets: RwLock::new(offsets),
        })
    }

    #[must_use]
    pub fn len(&self) -> u32 {
        self.offsets
            .read()
            .unwrap()
            .len()
            .try_into()
            .unwrap_or(u32::MAX)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub async fn line(&self, line: u32) -> Option<Line> {
        self.lines(line..=line).await.first().cloned()
    }

    #[must_use]
    pub async fn lines<R>(&self, range: R) -> Lines
    where
        R: RangeBounds<u32> + Send,
    {
        let offset = {
            let start = match range.start_bound().cloned() {
                Bound::Included(x) => x,
                Bound::Excluded(x) => x + 1,
                Bound::Unbounded => 0,
            } as usize;

            let Some(&v) = self.offsets.read().unwrap().get(start) else {
                return Lines::default();
            };

            v
        };

        let end = match range.end_bound().cloned() {
            Bound::Included(x) => x + 1,
            Bound::Excluded(x) => x,
            Bound::Unbounded => u32::MAX,
        } as usize;

        let limit = self
            .offsets
            .read()
            .unwrap()
            .get(end)
            .and_then(|v| v.checked_sub(offset))
            .and_then(|v| usize::try_from(v).ok());

        tracing::debug!("Reading lines {}:{offset}:{limit:?}", self.path.display());

        let Ok(file) = File::open(&self.path).await else {
            tracing::error!("Failed to read file {}", self.path.display());
            return Lines::default();
        };

        read_lines(file, offset, limit).await.unwrap_or_default()
    }

    pub async fn update(&self) -> Result<u32, Error> {
        if let Ok(index) = self.consistency().await?.into_inconsistent() {
            return Err(Error::InconsistentIndex(index));
        }

        let old_len = self.offsets.read().unwrap().len();
        let offset = self
            .offsets
            .read()
            .unwrap()
            .last()
            .copied()
            .unwrap_or_default();

        let mut file = File::open(&self.path).await?;
        let pos = file.seek(SeekFrom::Start(offset)).await?;
        assert_eq!(pos, offset);

        let offsets = spawn_blocking(move || index_lines(file)).await.unwrap()?;
        self.offsets.write().unwrap().extend(&offsets[1..]);

        Ok(self
            .offsets
            .read()
            .unwrap()
            .len()
            .checked_sub(old_len)
            .map(|v| v.try_into().unwrap_or(u32::MAX))
            .unwrap_or_default())
    }

    /// Verifies that the index is consistent with the file.
    /// Return `true` if the index is consistent, `false` otherwise.
    pub async fn consistency(&self) -> Result<IndexConsistency, Error> {
        let mut file = File::open(&self.path).await?;
        let file_len = file.metadata().await?.len();

        let offsets = self.offsets.read().unwrap().clone();

        for (index, &offset) in offsets.iter().enumerate().skip(1) {
            assert!(offset > 0);
            let offset = offset - 1;

            if offset > file_len {
                dbg!(1);
                return Ok(IndexConsistency::Inconsistent(index));
            }

            if offset != file.seek(SeekFrom::Start(offset)).await? {
                dbg!(2);
                return Ok(IndexConsistency::Inconsistent(index));
            }

            let byte = file.read_u8().await?;
            if b'\n' != byte {
                dbg!(byte as char);
                return Ok(IndexConsistency::Inconsistent(index));
            }
        }

        Ok(IndexConsistency::Consistent)
    }
}

#[derive(Debug, Clone, Copy, enum_as_inner::EnumAsInner, PartialEq, Eq)]
pub enum IndexConsistency {
    Consistent,
    Inconsistent(usize),
}

async fn read_lines(file: File, offset: u64, limit: Option<usize>) -> Result<Lines, Error> {
    let mut reader = BufReader::new(file);
    let pos = reader.seek(SeekFrom::Start(offset)).await?;
    assert_eq!(pos, offset);

    let buf = if let Some(limit) = limit {
        let mut buf = Vec::with_capacity(limit);
        reader.read_buf(&mut buf).await?;
        buf
    } else {
        // Dangerous!!! Reading without the limit.
        let mut buf = Vec::with_capacity(READ_BUF_CAPACITY);
        reader.read_to_end(&mut buf).await?;
        buf
    };

    // Reading from the mem buf, no need for async.
    std::io::BufReader::new(std::io::Cursor::new(buf))
        .lines()
        .map(|line| line.map(Into::into))
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
        .map_err(Into::into)
}

fn index_lines(file: File) -> Result<Vec<u64>, Error> {
    let mut file = file.try_into_std().unwrap();

    let mut offsets = vec![];

    let mut offset = file.stream_position()?;
    let mut buf = String::with_capacity(READ_BUF_CAPACITY);
    let mut reader = std::io::BufReader::new(&file);

    // TODO handle very long lines: read in chunks until the hard limit.
    while let Ok(read_bytes) = reader.read_line(&mut buf) {
        if read_bytes == 0 {
            break; // EOF
        }

        offsets.push(offset);

        if buf.chars().nth(read_bytes - 1) != Some('\n') {
            // No EOL, we've reached the end of the file.
            break;
        }
        buf.clear();

        offset += read_bytes as u64;

        assert_eq!(reader.stream_position()?, offset);
    }

    Ok(offsets)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Inconsistent index at line {0}")]
    InconsistentIndex(usize),
}
