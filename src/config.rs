use std::{collections::HashSet, fs::read_to_string, path::PathBuf};

use crossbeam::channel::{Receiver, unbounded};
use notify::{Error, Event, INotifyWatcher, RecursiveMode, Watcher, recommended_watcher};
use resolve_path::PathResolveExt;
use serde::Deserialize;

use crate::{JResult, bucket::Bucket, watch_path::WatchPath};

pub const DEFAULT_CONFIG_PATH: &str = "~/.config/janitors/config.toml";
type LoadConfigOutput = (Receiver<Result<Event, Error>>, Config);
type WatcherState = (Receiver<Result<Event, Error>>, WatchPath, INotifyWatcher);

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub watch: Vec<WatchPath>,
    pub bucket: Vec<Bucket>,
}

impl Config {
    pub fn load(file_path: &str) -> JResult<LoadConfigOutput> {
        let resolved_path = file_path.resolve();
        let config_str = read_to_string(&resolved_path)?;

        let mut config: Config = toml::from_str(&config_str)?;

        for b in config.bucket.iter_mut() {
            b.init()?;
        }

        let (tx, rx) = unbounded();
        let mut watcher = recommended_watcher(tx)?;
        watcher.watch(&PathBuf::from(resolved_path), RecursiveMode::NonRecursive)?;

        Ok((rx, config))
    }

    pub fn setup_watchers(
        &self,
        watchers: &mut Vec<WatcherState>,
        remove_indecies: &mut HashSet<usize>,
    ) -> JResult<()> {
        watchers.clear();
        remove_indecies.clear();

        for watch_path in self.watch.iter() {
            let (tx, rx) = unbounded();
            let mut watcher = recommended_watcher(tx)?;
            watcher.watch(&watch_path.path, watch_path.recursive_mode.into())?;

            // If the watcher gets dropped the channel closes, so we have to return it here.
            watchers.push((rx, watch_path.clone(), watcher));
        }

        Ok(())
    }

    pub fn one_shot(&self) -> JResult {
        for watch_path in self.watch.iter() {
            let recursive = matches!(
                watch_path.recursive_mode,
                crate::watch_path::RecMode::Recursive
            );
            let mut stack = vec![watch_path.path.clone()];
            let mut file_paths = Vec::new();
            let mut dir_paths = Vec::new();

            while let Some(p) = stack.pop() {
                if p.is_file() {
                    file_paths.push(p.clone());
                } else if p.is_dir() {
                    for dentry in p.read_dir()?.map_while(Result::ok) {
                        // Skip current and previous directory entries."
                        if let Some(fname) = dentry.path().file_name() {
                            if fname.to_string_lossy() == "." || fname.to_string_lossy() == ".." {
                                continue;
                            }
                        }
                        if recursive {
                            stack.push(dentry.path().clone());
                        } else if dentry.path().is_dir() {
                            dir_paths.push(dentry.path().clone());
                        } else if dentry.path().is_file() {
                            file_paths.push(dentry.path().clone())
                        }
                    }
                }
            }

            watch_path.handle_paths(file_paths.into_iter(), true, self)?;
            watch_path.handle_paths(dir_paths.into_iter(), false, self)?;
        }
        Ok(())
    }
}
