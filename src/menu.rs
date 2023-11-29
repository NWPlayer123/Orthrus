use argp::FromArgs;

/// Top-level command for argp to parse.
#[derive(FromArgs, Debug)]
#[argp(description = "A new way to modify games.")]
pub struct Orthrus {
    #[argp(option, short = 'v', global, default = "4")]
    #[argp(
        description = "Logging level (0 = FileOnly, 1 = Error, 2 = Warn, 3 = Info, 4 = Debug, 5 = Trace)"
    )]
    pub verbose: usize,

    #[argp(subcommand)]
    pub nested: Modules,
}

/// These are all the "modules" that Orthrus supports via command line.
#[derive(FromArgs, Debug)]
#[argp(subcommand)]
pub enum Modules {
    Yaz0(Yaz0Data),
    Panda3D(Panda3DOption),
}

/// Command-line flags for Yaz0 compression support
#[derive(FromArgs, Debug)]
#[argp(subcommand, name = "yaz0")]
#[argp(description = "Support for Nintendo's Yaz0 compression")]
pub struct Yaz0Data {
    #[argp(switch, short = 'd')]
    #[argp(description = "Decompress a Yaz0-compressed file")]
    pub decompress: bool,

    #[argp(switch, short = 'c')]
    #[argp(description = "Compress a binary file using Yaz0")]
    pub compress: bool,

    #[argp(option, short = 'i')]
    #[argp(description = "Input file to be processed")]
    pub input: String,

    #[argp(option, short = 'o')]
    #[argp(description = "Output file to write to")]
    pub output: String,
}

/// This is the command for the `Panda3D` module.
#[derive(FromArgs, Debug)]
#[argp(subcommand, name = "panda3d")]
#[argp(description = "Support for the Panda3D Engine")]
pub struct Panda3DOption {
    #[argp(subcommand)]
    pub nested: Panda3DModules,
}

/// These are all supported files within `Panda3D`.
#[derive(FromArgs, Debug)]
#[argp(subcommand)]
pub enum Panda3DModules {
    Multifile(MultifileData),
}

/// Command-line flags for `Panda3D` Multifile support
#[derive(FromArgs, Debug)]
#[argp(subcommand, name = "multifile")]
#[argp(description = "Panda3D Multifile Archive")]
pub struct MultifileData {
    #[argp(switch, short = 'x')]
    #[argp(description = "Extract all files from the Multifile")]
    pub extract: bool,

    #[argp(option, short = 'i')]
    #[argp(description = "Multifile to be processed")]
    pub input: String,

    #[argp(option, short = 'o')]
    #[argp(description = "Directory to extract to")]
    pub output: Option<String>,
}

#[must_use]
pub fn exactly_one_true(bools: &[bool]) -> Option<usize> {
    let mut count = 0;
    let mut index = 0;

    for (i, &val) in bools.iter().enumerate() {
        if val {
            count += 1;
            index = i;
        }

        if count > 1 {
            return None;
        }
    }

    if count == 1 {
        Some(index)
    } else {
        None
    }
}
