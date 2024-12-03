use argp::FromArgs;

use super::create_submodule;

create_submodule!(
    NintendoWare,
    "Support for Nintendo Middleware",
    BRSTM(BRSTMFlags),
    BFSAR(BFSARFlags)
);

#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argp(subcommand, name = "brstm")]
#[argp(description = "Binary File Stream")]
pub struct BRSTMFlags {
    #[argp(switch, short = 'd')]
    #[argp(description = "Decode the BRSTM into a WAV file")]
    pub decode: bool,

    #[argp(positional)]
    #[argp(description = "BRSTM file to be processed")]
    pub input: String,

    #[argp(positional)]
    #[argp(description = "WAV file to output to")]
    pub output: Option<String>,
}

#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argp(subcommand, name = "bfsar")]
#[argp(description = "Binary File Sound Archive")]
pub struct BFSARFlags {
    #[argp(switch, short = 'i')]
    #[argp(description = "Parse the BFSAR and print relevant information")]
    pub info: bool,

    #[argp(positional)]
    #[argp(description = "BFSAR to be processed")]
    pub input: String,
}
