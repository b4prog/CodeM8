use std::path::PathBuf;

mod args;
mod help;
mod version;

pub use args::{parse_args, parse_file_extensions, parse_file_list};
pub use help::help_text;
pub use version::codem8_version_from_cargo_lock;

use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Help,
    Report(CliConfig),
    Version,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    Duplicate,
    Complexity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliConfig {
    pub report: ReportKind,
    pub verbose: bool,
    pub file_extensions: Vec<String>,
    pub files: Option<Vec<PathBuf>>,
    pub git_branch: bool,
    pub max_cognitive_complexity: u32,
    pub max_cyclomatic_complexity: u32,
}

/// Parses command-line arguments into a CLI command.
///
/// # Errors
///
/// Returns an error when the arguments are invalid, repeated, or missing the
/// required report switch.
pub fn parse_command<I, S>(args: I) -> Result<CliCommand>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    if args.len() == 1 && is_help_argument(&args[0]) {
        return Ok(CliCommand::Help);
    }
    if args.len() == 1 && args[0] == "--version" {
        return Ok(CliCommand::Version);
    }
    parse_args(args).map(CliCommand::Report)
}

fn is_help_argument(arg: &str) -> bool {
    matches!(arg, "help" | "-h")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_help_command() {
        let command = parse_command(["help"]).expect("help parses");
        assert_eq!(command, CliCommand::Help);
    }

    #[test]
    fn parses_short_help_option() {
        let command = parse_command(["-h"]).expect("short help parses");
        assert_eq!(command, CliCommand::Help);
    }

    #[test]
    fn parses_version_option() {
        let command = parse_command(["--version"]).expect("version parses");
        assert_eq!(command, CliCommand::Version);
    }
}
