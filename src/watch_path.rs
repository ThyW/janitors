use crate::{bucket::Bucket, config::Config, errors::JResult};
use std::path::PathBuf;

use notify::{Event, EventKind, RecursiveMode};
use serde::Deserialize;

/// A `WatchPath` represents a path which is watched for new files.
///
/// When a new file is created in the given path, a supplied list of buckets is queried for a
/// fitting bucket. Once such a bucket is found, the bucket's action is applied on the file. If
/// the file fits into multiple buckets(even after comparing bucket priorities), the bucket with
/// the lowest lexicographical name is used. A recursive mode can also be provided, to either check
/// only the given directory(non-recursive) or the entire sub tree(recursive).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct WatchPath {
    /// Path to watch.
    pub path: PathBuf,
    /// Path recursive mode.
    pub recursive_mode: RecMode,
    /// Names of buckets to use.
    pub bucket_names: Vec<String>,
}

/// If the `Recursive` mode is used, the entire sub tree is watched for new files. If the
/// `NonRecursive` mode is used, only the immediate directory is checked for new files.
#[derive(Debug, Clone, Deserialize, Copy, Default, PartialEq, Eq)]
pub enum RecMode {
    #[serde(rename(deserialize = "recursive"))]
    Recursive,
    #[serde(rename(deserialize = "non-recursive"))]
    #[default]
    NonRecursive,
}

impl From<RecMode> for RecursiveMode {
    fn from(other: RecMode) -> RecursiveMode {
        match other {
            RecMode::Recursive => RecursiveMode::Recursive,
            RecMode::NonRecursive => RecursiveMode::NonRecursive,
        }
    }
}

impl WatchPath {
    /// Handle a provided file system event.
    pub fn handle_event(&self, ev: Event, config: &Config) -> JResult {
        if ev.attrs.flag().is_some() {
            // The `Rescan` flag has been found: ignore the event and re-scan.
            return Ok(());
        }
        let is_file = match ev.kind {
            EventKind::Create(create_kind) => match create_kind {
                notify::event::CreateKind::File => true,
                notify::event::CreateKind::Folder => false,
                _ => return Ok(()),
            },
            _ => return Ok(()),
        };
        log::trace!("Create event: {ev:?}");
        self.handle_paths(ev.paths.into_iter(), is_file, config)?;

        Ok(())
    }

    pub fn handle_paths<I>(&self, paths: I, is_file: bool, config: &Config) -> JResult
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let possible_buckets: Vec<&Bucket> = config
            .bucket
            .iter()
            .filter(|bucket| self.bucket_names.contains(&bucket.name))
            .collect();

        for path in paths.into_iter() {
            let mut fitting_buckets: Vec<&&Bucket> = possible_buckets
                .iter()
                .filter(|bucket| bucket.is_fitting(&path).is_ok_and(|inner| inner))
                .collect();
            fitting_buckets.sort();
            fitting_buckets.reverse();

            if let Some(bucket) = fitting_buckets.first() {
                log::trace!("picked bucket {} for file {}", bucket.name, &path.display());
                bucket.apply_action(&path, is_file)?;
            }
        }
        Ok(())
    }
}
