//! `falchion` binary entry point. Logic lives in the library for testability.

use std::process::ExitCode;

use clap::Parser;
use falchion_cli::{run, Cli};
use falchion_hid::HidProvider;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mut stdout = std::io::stdout().lock();
    match run(&cli, &HidProvider, &mut stdout) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
