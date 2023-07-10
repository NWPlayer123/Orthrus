use clap::{arg, crate_authors, crate_description, crate_version, ArgGroup, Command, Id};
use orthrus_yaz0 as yaz0;
use std::fs::File;
use std::io::prelude::*;

fn main() {
    let result = Command::new("Orthrus")
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .about(crate_description!())
        .arg_required_else_help(true)
        .subcommand(
            Command::new("yaz0")
                .about("Support for Nintendo's Yaz0 compression")
                .arg_required_else_help(true)
                .arg(arg!(-d --decompress "Decompress a Yaz0-compressed file"))
                .arg(arg!(<input> "Input file to be processed"))
                .arg(arg!(<output> "Output file to write to"))
                .group(
                    ArgGroup::new("methods")
                        .args(["decompress"])
                        .requires_all(["input", "output"])
                        .required(true),
                ),
        )
        .get_matches();

    match result.subcommand() {
        Some(("yaz0", sub_matches)) => {
            match sub_matches.get_one::<Id>("methods").unwrap().as_str() {
                "decompress" => {
                    let input_fn = sub_matches.get_one::<String>("input").unwrap();
                    let output_fn = sub_matches.get_one::<String>("output").unwrap();

                    let file = yaz0::decompress(input_fn).unwrap();
                    let mut output = File::create(output_fn).unwrap();
                    output.write_all(file.get_ref()).unwrap();
                }
                _ => unreachable!("Oops! Forgot to satisfy all methods"),
            }
        }
        _ => unreachable!("Oops! Forgot to satisfy all subcommands"),
    }
}
