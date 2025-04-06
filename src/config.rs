use std::{collections::HashSet, fs::read_to_string, path::PathBuf};

use crossbeam::channel::{Receiver, unbounded};
use notify::{Error, Event, RecursiveMode, Watcher, recommended_watcher};
use resolve_path::PathResolveExt;
use serde::Deserialize;

use crate::{JResult, bucket::Bucket, watch_path::WatchPath};

pub const DEFAULT_CONFIG_PATH: &str = "~/.config/janitors/config.toml";

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub watch_paths: Vec<WatchPath>,
    pub buckets: Vec<Bucket>,
}

type LoadConfigOutput = (Receiver<Result<Event, Error>>, Config);
impl Config {
    pub fn load() -> JResult<LoadConfigOutput> {
        let config_str = read_to_string(DEFAULT_CONFIG_PATH.resolve())?;

        let mut config: Config = toml::from_str(&config_str)?;

        for b in config.buckets.iter_mut() {
            b.init()?;
        }

        let (tx, rx) = unbounded();
        let mut watcher = recommended_watcher(tx)?;
        watcher.watch(
            &PathBuf::from(DEFAULT_CONFIG_PATH),
            RecursiveMode::NonRecursive,
        )?;

        Ok((rx, config))
    }

    pub fn setup_watchers(
        &self,
        watchers: &mut Vec<(Receiver<Result<Event, Error>>, WatchPath)>,
        remove_indecies: &mut HashSet<usize>,
    ) -> JResult<()> {
        watchers.clear();
        remove_indecies.clear();

        for watch_path in self.watch_paths.iter() {
            let (tx, rx) = unbounded();
            let mut watcher = recommended_watcher(tx)?;
            watcher.watch(&watch_path.path, watch_path.recursive_mode.into())?;

            watchers.push((rx, watch_path.clone()));
        }

        Ok(())
    }
}
