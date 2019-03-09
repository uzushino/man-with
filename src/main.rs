#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate failure;
extern crate man_with;
extern crate terminal_size;
extern crate termion;
extern crate unicode_width;

use std::process::Command;

use clap::{App, Arg};
use failure::Error;
use man_with::ManWith;

fn main() -> Result<(), Error> {
    env_logger::init();

    let matches = App::new("Man with a command")
        .version(crate_version!())
        .arg(
            Arg::with_name("COMMAND")
                .required(true)
                .help("Sets the man command.")
                .index(1),
        )
        .arg(
            Arg::with_name("size")
                .long("size")
                .short("s")
                .value_name("SIZE")
                .help("Sets the man viewer size.")
                .takes_value(true),
        )
        .get_matches();

    let command = matches.value_of("COMMAND").unwrap();
    let size = value_t!(matches, "size", usize).unwrap_or(10);
    let result = run(command, size)?;

    Command::new(result.0).args(result.1).spawn()?.wait()?;

    Ok(())
}

// When dropping raw mode stdout, return to original stdout.
fn run(command: &str, size: usize) -> Result<(String, Vec<String>), Error> {
    let app = ManWith::new(command, size);
    app.run()
}
