mod bucket;
mod config;
mod errors;
#[cfg(test)]
mod tests;
mod watch_path;

use clap::Parser;
use config::{CONFIG_PATHS, Config};
use crossbeam::channel::Select;
use notify::EventKind;
use resolve_path::PathResolveExt;
use std::{collections::HashSet, time::Duration};

use errors::JResult;

#[derive(Parser)]
struct Cli {
    #[arg(long, help = "run only once on all watch paths found in config")]
    one_shot: bool,
    #[arg(
        long,
        default_value_t = 5,
        help = "how verbose do we want to be with logs"
    )]
    verbosity: usize,
    config: Option<String>,
}

fn main() -> JResult {
    let cli = Cli::parse();
    // Initialize the logging facility.
    stderrlog::new()
        .verbosity(stderrlog::LogLevelNum::from(cli.verbosity))
        .timestamp(stderrlog::Timestamp::Second)
        .module(module_path!())
        .color(stderrlog::ColorChoice::Auto)
        .init()?;

    let config_file_path = if cli.config.is_some() {
        cli.config.unwrap()
    } else {
        let mut final_path = CONFIG_PATHS[2].into();
        for path in CONFIG_PATHS.iter() {
            if std::fs::exists(path.resolve())? {
                final_path = path.to_string();
                break;
            }
        }
        final_path
    };

    log::info!("using config: {}", config_file_path);

    let (mut rx, mut config) = Config::load(&config_file_path)?;
    log::info!("Loaded initial configuration.");
    if cli.one_shot {
        log::info!("Running in one-shot mode.");
        config.one_shot()?;
        return Ok(());
    }

    let mut watchers = Vec::new();
    let mut remove_indecies = HashSet::new();

    config.setup_watchers(&mut watchers, &mut remove_indecies)?;
    log::info!("File watchers have been setup.");
    let mut sel = Select::new();

    for (rx_, _, _) in watchers.iter() {
        sel.recv(rx_);
    }

    loop {
        if let Ok(Ok(ev)) = rx.try_recv() {
            if let EventKind::Modify(mev) = ev.kind {
                log::warn!(
                    "Config file '{}' has been modified.",
                    ev.paths.first().unwrap().display()
                );
                log::trace!("Config file modify event: {:?}", mev);
                let res = Config::load(&config_file_path);
                if let Err(e) = &res {
                    log::error!("reloading config: {e}");
                    log::warn!(
                        "config is not loaded, please fix the issues as soon as possible and save the config file to apply changes."
                    );
                    continue;
                }
                (rx, config) = res?;

                let res = config.setup_watchers(&mut watchers, &mut remove_indecies);
                if let Err(e) = &res {
                    log::error!("setting up file watchers: {}", e);
                }
                res?;

                sel = Select::new();
                for (rx_, _, _) in watchers.iter() {
                    sel.recv(rx_);
                }
            }
        }
        let res = sel.select_timeout(Duration::from_secs(1));
        if let Ok(op) = res {
            let idx = op.index();
            let (rx_, watch_path, _) = &watchers[idx];
            if remove_indecies.contains(&idx) {
                log::info!(
                    "Skipping event, because operation index '{}' is set to be ignored.",
                    idx
                );
                let _ = op.recv(rx_);
                continue;
            }

            let res = op.recv(rx_);
            match res {
                Ok(e) => {
                    let res = e;
                    if let Err(e) = &res {
                        log::error!("Notify event error: {e}");
                        continue;
                    }
                    let ev = res?;
                    let res = watch_path.handle_event(ev, &config);
                    if let Err(e) = &res {
                        log::error!(
                            "Error occured when handling event: {e}; make sure the destination path exists."
                        );
                        continue;
                    }
                    res?;
                }
                Err(e) => {
                    log::error!("Recv error received: {e}");
                    log::info!(
                        "Transmitter with index '{idx}' disconnected, ignoring all further messages."
                    );
                    remove_indecies.insert(idx);
                }
            }
        }
    }
}
