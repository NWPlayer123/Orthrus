use clap::{arg, crate_authors, crate_description, crate_version, ArgGroup, Command, Id};
use orthrus_panda3d as panda3d;
use orthrus_yaz0 as yaz0;
use owo_colors::OwoColorize;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use time::{macros::format_description, OffsetDateTime, UtcOffset};

/* Ideally I want this to function more like readable-log-formatter from Python but this works for now */
fn setup_logger(verbosity: u64) -> Result<(), fern::InitError> {
    let level = match verbosity {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Warn,
        2 => log::LevelFilter::Info,
        3 => log::LevelFilter::Debug,
        4 => log::LevelFilter::Trace,
        //default to highest
        _ => log::LevelFilter::Error,
    };

    //set up desired datetime format
    let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

    //initialize a base Dispatch we can apply our two profiles (output file and stdout) to
    let base_config = fern::Dispatch::new();

    let file_config = fern::Dispatch::new()
        .format(
            |out: fern::FormatCallback, message: &core::fmt::Arguments, record: &log::Record| {
                out.finish(format_args!(
                    "[{}] {:>5} {}",
                    OffsetDateTime::now_local().unwrap().format(format).unwrap(),
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

fn main() {
    //Enable ANSI support on Windows, ignore it if it fails for now
    match enable_ansi_support::enable_ansi_support() {
        Ok(()) => {}
        Err(_) => {}
    }

    setup_logger(4).unwrap();

    let result = Command::new("Orthrus")
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .about(crate_description!())
        .arg_required_else_help(true)
        .subcommand(
            Command::new("yaz0")
                .about("Support for Nintendo's Yaz0 compression")
                .arg_required_else_help(true)
                .arg(arg!(-d --decompress "Decompress a Yaz0-compressed file"))
                .arg(arg!(<input> "Input file to be processed"))
                .arg(arg!(<output> "Output file to write to"))
                .group(
                    ArgGroup::new("methods")
                        .args(["decompress"])
                        .requires_all(["input", "output"])
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("panda3d")
                .about("Support for the Panda3D Engine")
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("multifile")
                        .about("Panda3D Multifile Archive")
                        .arg_required_else_help(true)
                        .arg(arg!(-x --extract "Extract all files from the Multifile"))
                        .arg(arg!(<input> "Multifile to be read"))
                        .group(
                            ArgGroup::new("methods")
                                .args(["extract"])
                                .requires_all(["input"])
                                .required(true),
                        ),
                ),
        )
        .get_matches();

    match result.subcommand() {
        Some(("yaz0", sub_matches)) => {
            match sub_matches.get_one::<Id>("methods").unwrap().as_str() {
                "decompress" => {
                    let input_fn = sub_matches.get_one::<String>("input").unwrap();
                    let output_fn = sub_matches.get_one::<String>("output").unwrap();

                    let file = yaz0::decompress(input_fn).unwrap();
                    let mut output = File::create(output_fn).unwrap();
                    output.write_all(file.get_ref()).unwrap();
                }
                _ => unreachable!("Oops! Forgot to satisfy all methods"),
            }
        }
        Some(("panda3d", sub_matches)) => match sub_matches.subcommand() {
            Some(("multifile", sub_matches)) => {
                match sub_matches.get_one::<Id>("methods").unwrap().as_str() {
                    "extract" => {
                        let input_fn = sub_matches.get_one::<String>("input").unwrap();

                        let mut multifile = panda3d::Multifile::new();
                        multifile.open_read(Path::new(input_fn), 0).unwrap();
                    }
                    _ => unreachable!("Oops! forgot to satisfy all methods"),
                }
            }
            _ => unreachable!("Oops! Forgot to satisfy all subcommands"),
        },
        _ => unreachable!("Oops! Forgot to satisfy all subcommands"),
    }
}
