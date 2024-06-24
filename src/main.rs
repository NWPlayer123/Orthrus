use std::io::prelude::*;
use std::path::PathBuf;

use anyhow::Result;
use env_logger::Builder;
use log::{Level, LevelFilter};
use orthrus_jsystem::prelude::*;
use orthrus_ncompress::prelude::*;
use orthrus_nintendoware::prelude::*;
use orthrus_panda3d::prelude::*;
use owo_colors::OwoColorize;

mod identify;
mod menu;
use menu::{
    exactly_one_true, JSystemModules, Modules, NCompressModules, NintendoWareModules,
    Panda3DModules,
};

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

fn main() -> Result<()> {
    //Parse command line input
    let args: menu::Orthrus = argp::parse_args_or_exit(argp::DEFAULT);

    // Build up a logger with custom formatting and set it to the verbosity from the command line
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

    // Apologies for this mess, I care more about the crate usage than the command line parsing,
    // it'll get replaced by ui eventually
    match args.nested {
        Modules::IdentifyFile(params) => {
            crate::identify::identify_file(&params.input, params.deep_scan);
        }
        Modules::NintendoCompression(module) => match module.nested {
            NCompressModules::Yay0(params) => {
                match exactly_one_true(&[params.decompress, params.compress]) {
                    Some(0) => {
                        log::info!("Decompressing file {}", &params.input);
                        let data = Yay0::decompress_from_path(&params.input)?;
                        let output = if let Some(output) = params.output {
                            output
                        } else {
                            let mut new_path = PathBuf::from(params.input);
                            new_path.set_extension("arc");
                            new_path.to_string_lossy().into_owned()
                        };
                        log::info!("Writing file {}", output);
                        std::fs::write(output, data)?;
                    }
                    Some(1) => {
                        log::info!("Compressing file {}", &params.input);
                        let data = Yay0::compress_from_path(
                            &params.input,
                            yay0::CompressionAlgo::MatchingOld,
                            0,
                        )?;
                        let output = if let Some(output) = params.output {
                            output
                        } else {
                            let mut new_path = PathBuf::from(params.input);
                            new_path.set_extension("szp");
                            new_path.to_string_lossy().into_owned()
                        };
                        log::info!("Writing file {}", output);
                        std::fs::write(output, data)?;
                    }
                    None => eprintln!("Please select exactly one operation!"),
                    _ => unreachable!("Oops! Forgot to cover all operations."),
                }
            }
            NCompressModules::Yaz0(params) => {
                match exactly_one_true(&[params.decompress, params.compress]) {
                    Some(0) => {
                        log::info!("Decompressing file {}", &params.input);
                        let data = Yaz0::decompress_from_path(&params.input)?;
                        let output = if let Some(output) = params.output {
                            output
                        } else {
                            let mut new_path = PathBuf::from(params.input);
                            new_path.set_extension("arc");
                            new_path.to_string_lossy().into_owned()
                        };
                        log::info!("Writing file {}", output);
                        std::fs::write(output, data)?;
                    }
                    Some(1) => {
                        log::info!("Compressing file {}", &params.input);
                        let data = Yaz0::compress_from_path(
                            &params.input,
                            yaz0::CompressionAlgo::MatchingOld,
                            0,
                        )?;
                        let output = if let Some(output) = params.output {
                            output
                        } else {
                            let mut new_path = PathBuf::from(params.input);
                            new_path.set_extension("szs");
                            new_path.to_string_lossy().into_owned()
                        };
                        log::info!("Writing file {}", output);
                        std::fs::write(output, data)?;
                    }
                    None => eprintln!("Please select exactly one operation!"),
                    _ => unreachable!("Oops! Forgot to cover all operations."),
                }
            }
        },
        Modules::Panda3D(module) => match module.nested {
            Panda3DModules::Multifile(data) => {
                match exactly_one_true(&[data.extract]) {
                    Some(0) => {
                        // Ideally I could log each file path as it's written but I would have
                        // to refactor Multifile to use slice_take
                        let output = data.output.unwrap_or_else(|| ".".to_string());
                        Multifile::extract_from_path(data.input, output, 0)?;
                    }
                    None => eprintln!("Please select exactly one operation!"),
                    _ => unreachable!("Oops! Forgot to cover all operations."),
                }
            }
            Panda3DModules::BAM(data) => {
                BinaryAsset::open(data.input)?;
            }
        },
        Modules::JSystem(module) => match module.nested {
            JSystemModules::RARC(data) => {
                ResourceArchive::open(data.input)?;
            }
        },
        Modules::NintendoWare(module) => match module.nested {
            NintendoWareModules::BFSAR(data) => {
                Switch::BFSAR::open(data.input)?;
            }
        },
    }
    Ok(())
}
