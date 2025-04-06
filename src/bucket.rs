use std::{
    fs::{remove_dir_all, remove_file},
    path::{Path, PathBuf},
};

use fs_extra::{
    dir::{copy as copy_dir, move_dir},
    file::{copy, move_file},
};
use regex::Regex;
use serde::Deserialize;

use crate::errors::JResult;

/// A `Bucket` is a destination for files from watched paths.
///
/// It has filters for name and extensions. If a file 'fits' into multiple buckets, the one
/// with the highest `priority` is used. A bucket has multiple actions that can be
/// performed when a is being placed in it: the file can simply be moved, it can be
/// copied or deleted all together.
///
/// The `extension_filters` checks only the final extension, so for example file
/// `archive.tar.gz` would not be recognized by name filter `"tar"`, because only the final
/// extension is checked.
#[derive(Debug, Clone, Deserialize)]
pub struct Bucket {
    /// Unique identifier for the bucket.
    pub name: String,
    /// Where should the files belonging to this bucket be stored.
    pub destination: PathBuf,
    /// File extensions of files which should belong to this bucket.
    pub extension_filters: Vec<String>,
    /// Move the file into the bucket if the its name matches at least one of the filters.
    ///
    /// The filters use regular expressions.
    pub name_filters: Vec<String>,
    /// If multiple buckets can move a file, pick the one with the highest priority.
    pub priority: u32,
    /// What action should be performed on the file.
    pub action: Action,
    #[serde(skip)]
    _regexes: Vec<Regex>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    /// Move the file to the bucket destination.
    Move,
    /// Delete the file.
    Delete,
    /// Copy the file into the bucket destination.
    Copy,
}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for Bucket {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let ordering = self.priority.cmp(&other.priority);
        if ordering.is_eq() {
            return Some(self.name.cmp(&other.name));
        }
        Some(ordering)
    }
}

impl PartialEq for Bucket {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.destination == other.destination
            && self.priority == other.priority
    }
}

impl Eq for Bucket {}

impl Ord for Bucket {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Bucket {
    /// Given a path, check if the file fits into the bucket.
    pub fn is_fitting(&self, path: &impl AsRef<Path>) -> JResult<bool> {
        let path = path.as_ref();
        let opt = path.extension();
        if let Some(raw_ext) = opt {
            if let Some(extension) = raw_ext.to_str() {
                if self.extension_filters.contains(&extension.to_string()) {
                    return Ok(true);
                }
            }
        }
        // If no extension filters are not found, try name filters.
        let opt = path.file_name();
        if let Some(raw_fname) = opt {
            if let Some(fname) = raw_fname.to_str() {
                let name_match = self._regexes.iter().any(|filter| filter.is_match(fname));

                return Ok(name_match);
            }
        }
        Ok(false)
    }

    /// Try to apply the bucket's action on file.
    ///
    /// Note: This method does not check if the file fits into the bucket.
    pub fn apply_action(&self, path: &impl AsRef<Path>, is_file: bool) -> JResult {
        let path = path.as_ref();
        let to_path = self.destination.join(
            path.components()
                .next_back()
                .expect("unable to get last component of path"),
        );
        match self.action {
            Action::Delete => {
                if is_file {
                    remove_file(path)?
                } else {
                    remove_dir_all(path)?
                };
            }
            Action::Move => {
                if is_file {
                    move_file(
                        path,
                        to_path,
                        &fs_extra::file::CopyOptions::new().skip_exist(true),
                    )?;
                } else {
                    move_dir(path, to_path, &fs_extra::dir::CopyOptions::new())?;
                };
            }
            Action::Copy => {
                if is_file {
                    copy(path, to_path, &fs_extra::file::CopyOptions::new())?
                } else {
                    copy_dir(path, to_path, &fs_extra::dir::CopyOptions::new())?
                };
            }
        };

        Ok(())
    }

    /// Initialize Regex matchers.
    pub fn init(&mut self) -> JResult {
        self._regexes.clear();
        for filter in self.name_filters.iter() {
            self._regexes.push(Regex::new(filter)?);
        }

        Ok(())
    }
}
