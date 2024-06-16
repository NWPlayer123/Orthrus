use super::create_submodule;
use argp::FromArgs;

create_submodule!(
    Panda3D,
    "Support for the Panda3D Engine",
    Multifile(MultifileFlags),
    BAM(BAMFlags)
);

#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand, name = "multifile")]
#[argp(description = "Panda3D Multifile Archive")]
pub struct MultifileFlags {
    #[argp(switch, short = 'x')]
    #[argp(description = "Extract all files from the Multifile")]
    pub extract: bool,

    #[argp(positional)]
    #[argp(description = "Multifile to be processed")]
    pub input: String,

    #[argp(positional)]
    #[argp(description = "Directory to extract to")]
    pub output: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand, name = "bam")]
#[argp(description = "Panda3D Binary Model")]
pub struct BAMFlags {
    #[argp(switch, short = 'i')]
    #[argp(description = "Display info about the BAM file")]
    pub info: bool,

    #[argp(positional)]
    #[argp(description = "BAM file to be processed")]
    pub input: String,
}
