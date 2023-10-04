use std::fs::File;
use std::io::prelude::*;

use orthrus_core::{time, Result};
use orthrus_panda3d as panda3d;
use orthrus_yaz0::Yaz0;
use owo_colors::OwoColorize;

pub mod menu;
use menu::{exactly_one_true, Modules, Panda3DModules};

// Ideally I want this to function more like readable-log-formatter from Python but this works for
// now
fn setup_logger(verbose: usize) -> Result<()> {
    //Allow flexible logging level
    let level_filter = match verbose {
        0 => log::LevelFilter::Off,
        2 => log::LevelFilter::Warn,
        3 => log::LevelFilter::Info,
        4 => log::LevelFilter::Debug,
        5 => log::LevelFilter::Trace,
        //default to highest
        _ => log::LevelFilter::Error,
    };

    // initialize a base Dispatch we can apply our two profiles (output file and stdout) to
    let base_config = fern::Dispatch::new();

    let file_config = fern::Dispatch::new()
        .format(
            |out: fern::FormatCallback, message: &core::fmt::Arguments, record: &log::Record| {
                out.finish(format_args!(
                    "[{}] {:>5} {message}",
                    time::current_time().unwrap(),
                    record.level(), //display colors on console but not in the log file
                ));
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
                let formatted_message = format!("{}", message).replace('\n', "\n      ");
                out.finish(format_args!("{level:>16} {formatted_message}"));
            },
        )
        .level(level_filter)
        .chain(std::io::stdout());

    base_config.chain(file_config).chain(stdout_config).apply()?;

    match time::get_local_offset() {
        Ok(_) => {
            log::info!("Successfully set up logging using local timestamps");
        }
        Err(_) => {
            log::info!("Unable to acquire local offset, logging using UTC time!");
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    //Enable ANSI support on Windows, ignore it if it fails for now
    enable_ansi_support::enable_ansi_support()?;

    //Parse command line input
    let args: menu::Orthrus = argp::parse_args_or_exit(argp::DEFAULT);

    //Setup log and fern so we can output logging messages
    setup_logger(args.verbose)?;

    match args.nested {
        Modules::Yaz0(data) => {
            if let Some(index) = exactly_one_true(&[data.decompress]) {
                match index {
                    0 => {
                        let file = Yaz0::from_path(data.input)?;
                        let mut output = File::create(&data.output)?;
                        output.write_all(file.data.as_ref())?;
                    }
                    _ => unreachable!("Oops! Forgot to cover all operations."),
                }
            } else {
                log::error!("Please select exactly one operation!");
            }
        }
        Modules::Panda3D(module) => match module.nested {
            Panda3DModules::Multifile(data) => {
                if let Some(index) = exactly_one_true(&[data.extract]) {
                    match index {
                        0 => {
                            let _multifile = panda3d::Multifile::from_path(data.input, 0)?;
                        }
                        _ => unreachable!("Oops! Forgot to cover all operations."),
                    }
                } else {
                    log::error!("Please select exactly one operation!");
                }
            }
        },
    }
    Ok(())
}
