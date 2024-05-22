use std::{
    io::{BufRead, Seek, SeekFrom},
    ops::{Bound, RangeBounds},
    path::{Path, PathBuf},
};

use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, BufReader},
    spawn,
    sync::{mpsc, oneshot},
    task::spawn_blocking,
};

const READ_BUF_CAPACITY: usize = 8_192;

type LineRequest = (
    // Offset in the file, in bytes
    u64,
    // Limit reading in bytes. If `None`, read until the end of the file.
    Option<usize>,
    // Sender for the response
    oneshot::Sender<Result<Box<[String]>, Error>>,
);

pub struct LineIndexReader {
    path: PathBuf,
    offsets: Vec<u64>,
    tx: mpsc::Sender<LineRequest>,
}

/// Common interface
impl LineIndexReader {
    pub async fn index<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path> + Clone + Send,
    {
        let file = File::open(path.clone()).await?;
        let (file, offsets) = spawn_blocking(move || index_lines(file)).await.unwrap()?;

        let (tx, mut rx) = mpsc::channel::<LineRequest>(10);

        spawn(async move {
            let mut reader = BufReader::new(file);
            while let Some((offset, limit, resp)) = rx.recv().await {
                let result = read_lines(&mut reader, offset, limit).await;
                let _ = resp.send(result);
            }
        });

        Ok(Self {
            path: path.as_ref().to_owned(),
            offsets,
            tx,
        })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.offsets.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub async fn get_line(&self, line: usize) -> Option<String> {
        self.get_lines(line..=line)
            .await
            .and_then(|v| v.first().cloned())
    }

    #[must_use]
    pub async fn get_lines<R>(&self, range: R) -> Option<Box<[String]>>
    where
        R: RangeBounds<usize> + Send,
    {
        let offset = {
            let start = match range.start_bound() {
                Bound::Included(x) => *x,
                Bound::Excluded(x) => *x + 1,
                Bound::Unbounded => 0,
            };
            self.offsets.get(start).copied()?
        };

        let end = match range.end_bound() {
            Bound::Included(x) => Some(*x + 1),
            Bound::Excluded(x) => Some(*x),
            Bound::Unbounded => None,
        };

        let limit = end
            .and_then(|end| self.offsets.get(end))
            .or_else(|| self.offsets.last())
            .and_then(|v| v.checked_sub(offset))
            .and_then(|v| usize::try_from(v).ok());

        let (tx, rx) = oneshot::channel();
        self.tx.send((offset, limit, tx)).await.ok()?;

        rx.await.ok()?.ok()
    }

    pub async fn update(&mut self) -> Result<usize, Error> {
        if let Ok(index) = self.consistency().await?.into_inconsistent() {
            return Err(Error::InconsistentIndex(index));
        }

        let old_len = self.offsets.len();
        let offset = self.offsets.last().copied().unwrap_or_default();

        let mut file = File::open(&self.path).await?;
        let pos = file.seek(SeekFrom::Start(offset)).await?;
        assert_eq!(pos, offset);

        let (_, offsets) = spawn_blocking(move || index_lines(file)).await.unwrap()?;
        self.offsets.extend(&offsets[1..]);

        Ok(self.offsets.len() - old_len)
    }

    /// Verifies that the index is consistent with the file.
    /// Return `true` if the index is consistent, `false` otherwise.
    pub async fn consistency(&self) -> Result<IndexConsistency, Error> {
        let mut file = File::open(&self.path).await?;
        let file_len = file.metadata().await?.len();

        for (index, &offset) in self.offsets.iter().enumerate().skip(1) {
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

async fn read_lines(
    reader: &mut BufReader<File>,
    offset: u64,
    limit: Option<usize>,
) -> Result<Box<[String]>, Error> {
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
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
        .map_err(Into::into)
}

fn index_lines(file: File) -> Result<(File, Vec<u64>), Error> {
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

    file.rewind().unwrap();

    Ok((File::from_std(file), offsets))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Inconsistent index at line {0}")]
    InconsistentIndex(usize),
}
