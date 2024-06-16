use super::create_submodule;
use argp::FromArgs;

create_submodule!(
    NCompress,
    "Support for Nintendo compression formats",
    Yay0(Yay0Flags),
    Yaz0(Yaz0Flags)
);

#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand, name = "yay0")]
#[argp(description = "Nintendo Yay0-compressed data")]
pub struct Yay0Flags {
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

#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand, name = "yaz0")]
#[argp(description = "Nintendo Yaz0-compressed data")]
pub struct Yaz0Flags {
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
