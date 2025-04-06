mod bucket;
mod config;
mod errors;
#[cfg(test)]
mod tests;
mod watch_path;

use config::Config;
use crossbeam::channel::{Select, TryRecvError};
use notify::EventKind;
use std::{collections::HashSet, time::Duration};

use errors::JResult;

// TODO: add settings for file overriding/renaming/skipping

fn main() -> JResult {
    // Initialize the logging facility.
    stderrlog::new()
        .verbosity(stderrlog::LogLevelNum::Trace)
        .timestamp(stderrlog::Timestamp::Second)
        .module(module_path!())
        .color(stderrlog::ColorChoice::Auto)
        .init()?;

    let (mut rx, mut config) = Config::load()?;
    let mut watchers = Vec::new();
    let mut remove_indecies = HashSet::new();

    config.setup_watchers(&mut watchers, &mut remove_indecies)?;
    let mut sel = Select::new();

    for (rx, _) in watchers.iter() {
        sel.recv(rx);
    }

    loop {
        if let Ok(Ok(ev)) = rx.try_recv() {
            if let EventKind::Modify(mev) = ev.kind {
                log::warn!(
                    "Config file '{}' has been modified.",
                    ev.paths.first().unwrap().display()
                );
                log::trace!("Config file modify event: {:?}", mev);
                let res = Config::load();
                if let Err(e) = &res {
                    log::error!("reloading config: {e}");
                    continue;
                }
                (rx, config) = res?;

                let res = config.setup_watchers(&mut watchers, &mut remove_indecies);
                if let Err(e) = &res {
                    log::error!("setting up file watchers: {}", e);
                }
                res?;

                sel = Select::new();
                for (rx, _) in watchers.iter() {
                    sel.recv(rx);
                }
            }
        }
        let res = sel.select_timeout(Duration::from_secs(1));
        if let Ok(op) = res {
            let idx = op.index();
            if remove_indecies.contains(&idx) {
                log::info!(
                    "Skipping event, because operation index '{}' is set to be ignored.",
                    idx
                );
                continue;
            }

            let (rx_, watch_path) = &watchers[idx];
            match rx_.try_recv() {
                Ok(e) => {
                    let res = e;
                    if let Err(e) = &res {
                        log::error!("Notify event error: {e}");
                        continue;
                    }
                    let ev = res?;
                    log::trace!("Notify event: {:?}", ev);
                    let res = watch_path.handle(ev, &config);
                    if let Err(e) = &res {
                        log::error!("Error occured when handling event: {e}");
                        continue;
                    }
                    res?;
                }
                Err(e) => {
                    if e == TryRecvError::Disconnected {
                        log::info!(
                            "Transmitter with index '{idx}' disconnected, ignoring all further messages."
                        );
                        remove_indecies.insert(idx);
                    }
                }
            }
        }
    }
}
