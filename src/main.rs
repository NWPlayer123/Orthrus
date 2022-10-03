use std::fs::File;
use std::io::prelude::*;

use clap::{AppSettings, Arg, Command, SubCommand};

fn main() {
    //TODO: allow modules to generate their own subcommand and just register it here
    let matches = Command::new("Orthrus")
    .about("A new way to modify games.")
    .setting(AppSettings::ArgRequiredElseHelp)
    .subcommand(SubCommand::with_name("decompress")
                .arg(Arg::with_name("input")
                    .required(true))
                .arg(Arg::with_name("output")
                    .required(true)))
    .get_matches();

    if let Some(matches) = matches.subcommand_matches("decompress") {
        //unwrap, it is required and will always be a string
        let input_fn = matches.get_one::<String>("input").unwrap();
        let output_fn = matches.get_one::<String>("output").unwrap();

        let file = yaz0::load(input_fn).unwrap();
        let mut output = File::create(output_fn).unwrap();
        output.write(&file).unwrap();
    }
}
