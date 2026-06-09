use clap::Parser;
use std::process::ExitCode;

mod cli;
mod detect;
mod error;
mod optimize;
mod pipeline;
mod report;
mod safety;

use cli::Args;

fn main() -> ExitCode {
    let args = Args::parse();

    match pipeline::run(args) {
        Ok(()) => ExitCode::from(0),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
