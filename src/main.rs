use std::fs::File;
use std::io::prelude::*;

use env_logger::Builder;
use log::{Level, LevelFilter};
use orthrus_ncompress::*;
use orthrus_panda3d::prelude as panda3d;
use owo_colors::OwoColorize;

mod menu;
use menu::{exactly_one_true, Modules, Panda3DModules};

fn color_level(level: Level) -> String {
    match level {
        Level::Error => level.red().to_string(),
        Level::Warn => level.yellow().to_string(),
        Level::Info => level.green().to_string(),
        Level::Debug => level.blue().to_string(),
        Level::Trace => level.purple().to_string(),
    }
}

const fn level_filter(verbose: usize) -> LevelFilter {
    match verbose {
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        4 => LevelFilter::Debug,
        5 => LevelFilter::Trace,
        //default to off
        _ => LevelFilter::Off,
    }
}

fn main() {
    //Parse command line input
    let args: menu::Orthrus = argp::parse_args_or_exit(argp::DEFAULT);

    //This is needed for pretty colors on Windows
    enable_ansi_support::enable_ansi_support()
        .expect("Please update to a modern operating system.");

    //Build up a logger with custom formatting and set it to the verbosity from the command line
    // args
    if args.verbose != 0 {
        Builder::new()
            .format(|buf, record| {
                writeln!(
                    buf,
                    "[{}] {} {}",
                    orthrus_core::time::current_time(), // Use your custom time function
                    color_level(record.level()),        // Colored log level
                    record.args()                       // Log message
                )
            })
            .filter(None, level_filter(args.verbose))
            .init();
    }

    match args.nested {
        Modules::Yaz0(params) => {
            if let Some(index) = exactly_one_true(&[params.decompress, params.compress]) {
                match index {
                    0 => {
                        let data = yaz0::decompress_from_path(params.input).unwrap();
                        let mut output = File::create(params.output).unwrap();
                        output.write_all(&data).unwrap();
                    }
                    1 => {
                        let data = yaz0::compress_from_path(
                            params.input,
                            yaz0::CompressionAlgo::MatchingOld,
                            0,
                        )
                        .unwrap();
                        let mut output = File::create(params.output).unwrap();
                        output.write_all(&data).unwrap();
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
                            panda3d::Multifile::extract_from_path(
                                data.input,
                                data.output.unwrap_or_else(|| ".".to_string()),
                                0,
                            )
                            .unwrap();
                        }
                        _ => unreachable!("Oops! Forgot to cover all operations."),
                    }
                } else {
                    log::error!("Please select exactly one operation!");
                }
            }
        },
    }
}
