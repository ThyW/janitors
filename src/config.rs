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
}
