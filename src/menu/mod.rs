use argp::FromArgs;

mod jsystem;
pub use jsystem::JSystemModules;
use jsystem::JSystemOption;
mod ncompress;
pub use ncompress::NCompressModules;
use ncompress::NCompressOption;
mod nintendoware;
pub use nintendoware::NintendoWareModules;
use nintendoware::NintendoWareOption;
mod panda3d;
pub use panda3d::Panda3DModules;
use panda3d::Panda3DOption;

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
    JSystem(JSystemOption),
    NintendoWare(NintendoWareOption),
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

// Some interaction with argp/argh's derives breaks doc comment macro expansion, so I can't use
// `#[doc = concat!("", stringify!($module_str), "")]`
macro_rules! create_submodule {
    ($module_name:ident, $module_description:expr, $( $submodule_name:ident($submodule_type:ty) ),* ) => {
        use paste::paste;
        paste! {
            // This is the command for the `$module_str` module.
            #[derive(FromArgs, PartialEq, Debug)]
            #[argp(subcommand, name = $module_name:lower)]
            #[argp(description = $module_description)]
            pub struct [<$module_name Option>] {
                #[argp(subcommand)]
                pub nested: [<$module_name Modules>],
            }

            // These are all supported files within `$module_str`.
            #[derive(FromArgs, PartialEq, Debug)]
            #[argp(subcommand)]
            #[non_exhaustive]
            pub enum [<$module_name Modules>] {
                $( $submodule_name($submodule_type) ),*
            }
        }
    };
}
pub(crate) use create_submodule;
