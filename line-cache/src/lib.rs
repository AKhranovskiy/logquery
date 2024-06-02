use std::{ops::RangeBounds, sync::Arc};

use itertools::Itertools;
use mini_moka::sync::Cache;

use line_index_reader::LineIndexReader;

pub type Index = u32;
pub type Line = Arc<str>;
pub type Lines = Box<[Line]>;

pub struct LineCache {
    reader: Arc<LineIndexReader>,
    cache: Arc<Cache<Index, Line>>,
}

// TODO make cache capacity configurable.
const CACHE_MAX_CAPACITY: u64 = 256 * 1024 * 1024; // 256MB

impl LineCache {
    #[must_use]
    pub fn new(reader: Arc<LineIndexReader>) -> Self {
        let cache = Arc::new(
            Cache::builder()
                .weigher(|_, value: &Line| {
                    value
                        .len()
                        .try_into()
                        .unwrap_or(u32::MAX)
                        .clamp(1, u32::MAX)
                })
                .max_capacity(CACHE_MAX_CAPACITY)
                .build(),
        );

        Self { reader, cache }
    }

    pub async fn line(&self, index: u32) -> Option<Line> {
        if let Some(line) = self.cache.get(&index) {
            Some(line)
        } else {
            self.lines(index..index).await.first().cloned()
        }
    }

    pub async fn lines<R>(&self, range: R) -> Lines
    where
        R: RangeBounds<u32> + Send,
    {
        let start = match range.start_bound().cloned() {
            std::ops::Bound::Included(i) => i,
            std::ops::Bound::Excluded(i) => i + 1,
            std::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound().cloned() {
            std::ops::Bound::Included(i) => i + 1,
            std::ops::Bound::Excluded(i) => i,
            std::ops::Bound::Unbounded => u32::MAX,
        };

        tracing::debug!("Fetching lines {start}:{end} from cache");

        let cached_lines = (start..end)
            .map_while(|index| self.cache.get(&index))
            .collect_vec();

        let len = cached_lines.len().try_into().unwrap_or(u32::MAX);

        tracing::debug!("Found {start}:{} from {start}:{end} in cache", start + len);

        let range = start + len..end;
        if range.is_empty() {
            return cached_lines.into_boxed_slice();
        }

        let len = end - start;
        // TODO pre-fetch lines before range if they are not in cache.
        let prefetch = range.start..range.end.saturating_add(len.saturating_mul(10).min(2_048));

        tracing::debug!("Fetching {}:{} from file", prefetch.start, prefetch.end);

        let new_lines: Vec<Line> = self
            .reader
            .lines(prefetch.clone())
            .await
            .into_vec()
            .into_iter()
            .map(Line::from)
            .collect_vec();

        tracing::debug!(
            "Read {}:{} from file",
            prefetch.start,
            prefetch.start + new_lines.len().try_into().unwrap_or(u32::MAX)
        );

        for (index, line) in prefetch.zip(&new_lines) {
            self.cache.insert(index, line.clone());
        }

        let mut lines = cached_lines;
        lines.extend(new_lines.into_iter().take(range.len()));
        lines.into_boxed_slice()
    }

    pub fn lines_opt<R>(&self, range: R) -> Box<[Option<Line>]>
    where
        R: RangeBounds<u32> + Send,
    {
        let start = match range.start_bound().cloned() {
            std::ops::Bound::Included(i) => i,
            std::ops::Bound::Excluded(i) => i + 1,
            std::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound().cloned() {
            std::ops::Bound::Included(i) => i + 1,
            std::ops::Bound::Excluded(i) => i,
            std::ops::Bound::Unbounded => u32::MAX,
        };

        tracing::trace!("Fetching lines {start}:{end} from cache");

        (start..end)
            .map(|index| self.cache.get(&index))
            .collect_vec()
            .into_boxed_slice()
    }
}
