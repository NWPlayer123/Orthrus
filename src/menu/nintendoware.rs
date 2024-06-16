use super::create_submodule;
use argp::FromArgs;

create_submodule!(
    NintendoWare,
    "Support for Nintendo Middleware",
    BFSAR(BFSARFlags)
);

#[derive(FromArgs, PartialEq, Debug)]
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
