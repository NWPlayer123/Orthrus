use argp::FromArgs;

use super::create_submodule;

create_submodule!(
    JSystem,
    "Support for Nintendo's JSystem Middleware",
    RARC(RARCFlags)
);

#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand, name = "rarc")]
#[argp(description = "JSystem Resource Archive")]
pub struct RARCFlags {
    #[argp(switch, short = 'x')]
    #[argp(description = "Extract all files from the RARC")]
    pub extract: bool,

    //Extract requires output so just ask for both
    #[argp(positional)]
    #[argp(description = "RARC to be processed")]
    pub input: String,

    #[argp(positional)]
    #[argp(description = "Directory to extract to")]
    pub output: Option<String>,
}
