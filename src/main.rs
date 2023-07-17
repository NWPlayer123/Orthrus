use orthrus_helper as orthrus;
use orthrus_helper::Result;
use orthrus_panda3d as panda3d;
use orthrus_yaz0 as yaz0;

use owo_colors::OwoColorize;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use time::{OffsetDateTime, UtcOffset};

pub mod menu;
use menu::*;

/* Ideally I want this to function more like readable-log-formatter from Python but this works for now */
fn setup_logger(verbosity: u64) -> Result<()> {
    let level = match verbosity {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Warn,
        2 => log::LevelFilter::Info,
        3 => log::LevelFilter::Debug,
        4 => log::LevelFilter::Trace,
        //default to highest
        _ => log::LevelFilter::Error,
    };

    //initialize a base Dispatch we can apply our two profiles (output file and stdout) to
    let base_config = fern::Dispatch::new();

    let file_config = fern::Dispatch::new()
        .format(
            |out: fern::FormatCallback, message: &core::fmt::Arguments, record: &log::Record| {
                out.finish(format_args!(
                    "[{}] {:>5} {}",
                    orthrus::current_time().unwrap(),
                    record.level(), //display colors on console but not in the log file
                    message
                ))
            },
        )
        .chain(fern::log_file("output.log")?);

    let stdout_config = fern::Dispatch::new()
        .format(
            |out: fern::FormatCallback, message: &core::fmt::Arguments, record: &log::Record| {
                let level = match record.level() {
                    log::Level::Error => record.level().red().to_string(),
                    log::Level::Warn => record.level().yellow().to_string(),
                    log::Level::Info => record.level().green().to_string(),
                    log::Level::Debug => record.level().blue().to_string(),
                    log::Level::Trace => record.level().purple().to_string(),
                };
                out.finish(format_args!(
                    "{:>16} {}",
                    level, //display colors on console but not in the log file
                    message
                ))
            },
        )
        .level(level)
        .chain(std::io::stdout());

    base_config
        .chain(file_config)
        .chain(stdout_config)
        .apply()?;

    match UtcOffset::local_offset_at(OffsetDateTime::UNIX_EPOCH) {
        Ok(_) => {
            log::info!("Successfully set up logging using local timestamps")
        }
        Err(_) => {
            log::info!("Unable to acquire local offset, logging using UTC time!")
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    //Enable ANSI support on Windows, ignore it if it fails for now
    match enable_ansi_support::enable_ansi_support() {
        Ok(()) => {}
        Err(_) => {}
    }

    setup_logger(4).unwrap();

    let args: TopLevel = argp::parse_args_or_exit(argp::DEFAULT);

    match args.nested {
        Modules::Yaz0(data) => match exactly_one_true(&[data.decompress]) {
            Some(index) => match index {
                0 => {
                    let file = yaz0::decompress(&data.input)?;
                    let mut output = File::create(&data.output)?;
                    output.write_all(file.get_ref())?;
                }
                _ => unreachable!("Oops! Forgot to cover all operations."),
            },
            None => log::error!("Please select exactly one operation!"),
        },
        Modules::Panda3D(module) => match module.nested {
            Panda3DModules::Multifile(data) => match exactly_one_true(&[data.extract]) {
                Some(index) => match index {
                    0 => {
                        let mut multifile = panda3d::Multifile::new();
                        match multifile.open_read(Path::new(&data.input), 0) {
                            Ok(_) => {}
                            Err(_) => {}
                        };
                    }
                    _ => unreachable!("Oops! Forgot to cover all operations."),
                },
                None => log::error!("Please select exactly one operation!"),
            },
        },
    }
    Ok(())
}
