use argp::FromArgs;

use super::create_submodule;

create_submodule!(
    Godot,
    "Support for the Godot game engine",
    Godot(GodotFlags)
);

#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argp(subcommand, name = "pck")]
#[argp(description = "Godot Resource Pack")]
pub struct GodotFlags {
    #[argp(switch, short = 'x')]
    #[argp(description = "Extract all files from the PCK")]
    pub extract: bool,

    //Extract requires output so just ask for both
    #[argp(positional)]
    #[argp(description = "PCK to be processed")]
    pub input: String,

    #[argp(positional)]
    #[argp(description = "Directory to extract to")]
    pub output: Option<String>,
}
