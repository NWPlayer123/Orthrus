use argp::FromArgs;
mod ncompress;
use ncompress::NCompressOption;
pub use ncompress::NCompressModules;
mod panda3d;
use panda3d::Panda3DOption;
pub use panda3d::Panda3DModules;

/// Top-level command
#[derive(FromArgs, PartialEq, Debug)]
#[argp(description = "A new way to modify games.")]
pub struct Orthrus {
    #[argp(option, short = 'v', global, default = "0")]
    #[argp(
        description = "Logging level (0 = Off, 1 = Error, 2 = Warn, 3 = Info, 4 = Debug, 5 = Trace)"
    )]
    pub verbose: usize,

    #[argp(subcommand)]
    pub nested: Modules,
}

/// These are all the "modules" that Orthrus supports via command line.
#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand)]
#[non_exhaustive]
pub enum Modules {
    IdentifyFile(IdentifyOption),
    NintendoCompression(NCompressOption),
    Panda3D(Panda3DOption),
}

/// Command to try to identify what a given file is.
#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand, name = "info")]
#[argp(description = "Identify a file and print relevant information")]
pub struct IdentifyOption {
    #[argp(switch, long = "deep")]
    #[argp(description = "Allow Orthrus to do more compute-intensive operations when scanning.")]
    pub deep_scan: bool,

    //We always need an input file, output file can be optional with a default
    #[argp(positional)]
    #[argp(description = "Input file to be processed")]
    pub input: String,
}

#[must_use]
pub fn exactly_one_true(bools: &[bool]) -> Option<usize> {
    let mut count: usize = 0;
    let mut index: usize = 0;

    for (i, &val) in bools.iter().enumerate() {
        if val {
            count += 1;
            index = i;
        }

        if count > 1 {
            break;
        }
    }

    (count == 1).then_some(index)
}
