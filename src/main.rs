#![allow(clippy::multiple_crate_versions)]

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let current_dir = match std::env::current_dir() {
        Ok(current_dir) => current_dir,
        Err(error) => {
            eprintln!("error: could not determine current directory: {error}");
            return ExitCode::FAILURE;
        }
    };
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    match codem8::run(std::env::args().skip(1), &current_dir, &mut stdout) {
        Ok(()) => {
            let _ = stdout.flush();
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            if error.should_show_help() {
                eprintln!();
                eprint!("{}", codem8::cli::help_text());
            }
            ExitCode::FAILURE
        }
    }
}
