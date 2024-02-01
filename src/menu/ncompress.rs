use argp::FromArgs;

/// This is the command for the `ncompress` module.
#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand, name = "ncompress")]
#[argp(description = "Support for Nintendo compression formats")]
pub struct NCompressOption {
    #[argp(subcommand)]
    pub nested: NCompressModules,
}

/// These are all supported types within `ncompress`.
#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand)]
#[non_exhaustive]
pub enum NCompressModules {
    Yay0(Yay0Data),
    Yaz0(Yaz0Data),
}

/// Command-line flags for Yaz0 compression support
#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand, name = "yay0")]
#[argp(description = "Nintendo Yay0-compressed Data")]
pub struct Yay0Data {
    #[argp(switch, short = 'd')]
    #[argp(description = "Decompress a Yay0-compressed file")]
    pub decompress: bool,

    #[argp(switch, short = 'c')]
    #[argp(description = "Compress a binary file using Yay0")]
    pub compress: bool,

    //We always need an input file, output file can be optional with a default
    #[argp(positional)]
    #[argp(description = "Input file to be processed")]
    pub input: String,

    #[argp(positional)]
    #[argp(description = "Output file to write to")]
    pub output: Option<String>,
}

/// Command-line flags for Yaz0 compression support
#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand, name = "yaz0")]
#[argp(description = "Nintendo Yaz0-compressed Data")]
pub struct Yaz0Data {
    #[argp(switch, short = 'd')]
    #[argp(description = "Decompress a Yaz0-compressed file")]
    pub decompress: bool,

    #[argp(switch, short = 'c')]
    #[argp(description = "Compress a binary file using Yaz0")]
    pub compress: bool,

    //We always need an input file, output file can be optional with a default
    #[argp(positional)]
    #[argp(description = "Input file to be processed")]
    pub input: String,

    #[argp(positional)]
    #[argp(description = "Output file to write to")]
    pub output: Option<String>,
}
