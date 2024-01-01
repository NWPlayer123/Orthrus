use argp::FromArgs;

/// This is the command for the `Panda3D` module.
#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand, name = "panda3d")]
#[argp(description = "Support for the Panda3D Engine")]
pub struct Panda3DOption {
    #[argp(subcommand)]
    pub nested: Panda3DModules,
}

/// These are all supported files within `Panda3D`.
#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand)]
#[non_exhaustive]
pub enum Panda3DModules {
    Multifile(MultifileData),
}

/// Command-line flags for `Panda3D` Multifile support
#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand, name = "multifile")]
#[argp(description = "Panda3D Multifile Archive")]
pub struct MultifileData {
    #[argp(switch, short = 'x')]
    #[argp(description = "Extract all files from the Multifile")]
    pub extract: bool,

    //Extract requires output so just ask for both
    #[argp(positional)]
    #[argp(description = "Multifile to be processed")]
    pub input: String,

    #[argp(positional)]
    #[argp(description = "Directory to extract to")]
    pub output: Option<String>,
}
