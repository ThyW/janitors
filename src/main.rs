mod bucket;
mod config;
mod watch_path;

use config::Config;
use crossbeam::channel::{Select, TryRecvError};
use notify::EventKind;
use std::time::Duration;

// TODO: proper error handling
// TODO: add settings for file overriding/renaming/skipping

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut rx, mut config) = Config::load()?;
    let mut watchers = Vec::new();
    let mut remove_indecies = Vec::new();

    config.setup_watchers(&mut watchers, &mut remove_indecies)?;
    let mut sel = Select::new();

    for (rx, _) in watchers.iter() {
        sel.recv(rx);
    }

    loop {
        if let Ok(Ok(ev)) = rx.try_recv() {
            if let EventKind::Modify(_) = ev.kind {
                (rx, config) = Config::load()?;

                config.setup_watchers(&mut watchers, &mut remove_indecies)?;
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
                continue;
            }

            let (rx_, watch_path) = &watchers[idx];
            match rx_.try_recv() {
                Ok(e) => {
                    let ev = e?;
                    watch_path.handle(ev, &config)?;
                }
                Err(e) => {
                    if e == TryRecvError::Disconnected {
                        remove_indecies.push(idx);
                    }
                }
            }
        }
    }
}
