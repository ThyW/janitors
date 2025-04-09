use std::{
    fs::{remove_dir_all, remove_file},
    path::{Path, PathBuf},
};

use anyhow::bail;
use fs_extra::{
    dir::{copy as copy_dir, move_dir},
    file::{copy, move_file},
};
use regex::Regex;
use serde::Deserialize;

use crate::errors::{JError, JResult};

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
#[derive(Debug, Clone, Deserialize, Default)]
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
    /// What action should be taken, if a file/directory of the same name exists in the bucket
    /// already.
    pub override_action: OverrideAction,
    #[serde(skip)]
    pub _regexes: Vec<Regex>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Move the file to the bucket destination.
    #[default]
    Move,
    /// Delete the file.
    Delete,
    /// Copy the file into the bucket destination.
    Copy,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
#[serde(rename_all = "lowercase")]
pub enum OverrideAction {
    /// Overwrite the file/directory replacing its contents with the contents of the new file.
    Overwrite,
    /// Try to rename the file in sequential order `file.txt` -> `file.txt.1` -> `file.txt.2`
    Rename,
    #[default]
    Skip,
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

    // Rename path semantically.
    fn rename_seq(&self, path: &impl AsRef<Path>) -> JResult<PathBuf> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(path.to_owned());
        }
        let mut count = 1;
        let mut other_path = path.to_owned();

        while other_path.exists() {
            if let Some(path_str) = path.to_str() {
                other_path = PathBuf::from(format!("{}.{}", path_str, count));
                count += 1;
                continue;
            }
            bail!(JError::InvalidPath(other_path))
        }

        log::info!(
            "sequentially renamed '{}' to {}",
            path.display(),
            other_path.display()
        );
        Ok(other_path)
    }

    /// Try to apply the bucket's action on file.
    ///
    /// Note: This method does not check if the file fits into the bucket.
    pub fn apply_action(&self, path: &impl AsRef<Path>, is_file: bool) -> JResult {
        let path = path.as_ref();
        let mut to_path = self.destination.join(
            path.components()
                .next_back()
                .expect("unable to get last component of path"),
        );

        if matches!(self.override_action, OverrideAction::Skip)
            && to_path.exists()
            && !matches!(self.action, Action::Delete)
        {
            log::info!(
                "skipping '{}' because bin action is 'skip' and '{}' already exists",
                path.display(),
                to_path.display(),
            );
            return Ok(());
        }

        if matches!(self.override_action, OverrideAction::Rename) {
            to_path = self.rename_seq(&to_path)?;
        }

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

        log::info!("'{}' put into bin '{}'.", path.display(), self.name);

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
