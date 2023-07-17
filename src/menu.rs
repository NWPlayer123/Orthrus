use argp::FromArgs;

/// A new way to modify games.
#[derive(FromArgs, Debug)]
pub struct TopLevel {
    /// Be verbose.
    #[argp(switch, short = 'v', global)]
    pub verbose: bool,

    /// Run locally.
    #[argp(switch)]
    pub quiet: bool,

    #[argp(subcommand)]
    pub nested: Modules,
}

#[derive(FromArgs, Debug)]
#[argp(subcommand)]
pub enum Modules {
    Yaz0(Yaz0Data),
    Panda3D(Panda3DOption),
}

/// Support for Nintendo's Yaz0 compression
#[derive(FromArgs, Debug)]
#[argp(subcommand, name = "yaz0")]
pub struct Yaz0Data {
    /// Decompress a Yaz0-compressed file
    #[argp(switch, short = 'd')]
    pub decompress: bool,

    /// Input file to be processed
    #[argp(option, short = 'i')]
    pub input: String,

    /// Output file to write to
    #[argp(option, short = 'o')]
    pub output: String,
}

/// Support for the Panda3D Engine
#[derive(FromArgs, Debug)]
#[argp(subcommand, name = "panda3d")]
pub struct Panda3DOption {
    #[argp(subcommand)]
    pub nested: Panda3DModules,
}

#[derive(FromArgs, Debug)]
#[argp(subcommand)]
pub enum Panda3DModules {
    Multifile(MultifileData),
}

/// Panda3D Multifile Archive
#[derive(FromArgs, Debug)]
#[argp(subcommand, name = "multifile")]
pub struct MultifileData {
    /// Extract all files from the Multifile
    #[argp(switch, short = 'x')]
    pub extract: bool,

    /// Multifile to be processed
    #[argp(option, short = 'i')]
    pub input: String,
}

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
