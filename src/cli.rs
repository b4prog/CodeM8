use std::path::PathBuf;

use crate::error::{CodeM8Error, Result};
use crate::language::supported_file_extensions;

const HELP_TEXT: &str = "\
CodeM8 - deterministic source code analysis reports.

USAGE:
  codem8 help
  codem8 --report-duplicate [OPTIONS]

COMMANDS:
  help
      Display this detailed documentation.

REQUIRED REPORT SWITCHES:
  --report-duplicate
      Analyze source files and print a duplicate code report.

OPTIONS:
  -file-extension=<extensions>
  --file-extension=<extensions>
      Comma-separated source file extensions to analyze.
      Defaults to all extensions registered in LANGUAGE_PATTERNS.
      Examples: -file-extension=ts,tsx,js,jsx

  -files=<paths>
  --files=<paths>
      Comma-separated explicit files to analyze instead of recursively
      discovering files from the current directory.
      Example: -files=src/a.ts,src/b.js

DUPLICATE REPORT PURPOSE:
  The duplicate report helps you find repeated code that may be worth
  refactoring, reviewing, or consolidating. It lists each duplicated block with
  the files and line ranges where it appears, making it easier to compare the
  repeated code and decide whether it should stay duplicated.

EXAMPLES:
  codem8 --report-duplicate
  codem8 --report-duplicate -file-extension=ts,tsx,js,jsx
  codem8 --report-duplicate -file-extension=ts,js -files=src/a.ts,src/b.js
";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Help,
    ReportDuplicate(CliConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliConfig {
    pub report_duplicate: bool,
    pub file_extensions: Vec<String>,
    pub files: Option<Vec<PathBuf>>,
}

#[must_use]
pub const fn help_text() -> &'static str {
    HELP_TEXT
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
    parse_args(args).map(CliCommand::ReportDuplicate)
}

/// Parses command-line arguments into a validated CLI configuration.
///
/// # Errors
///
/// Returns an error when the arguments are invalid, repeated, or missing the
/// required report switch.
pub fn parse_args<I, S>(args: I) -> Result<CliConfig>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut report_duplicate = false;
    let mut file_extensions = None;
    let mut files = None;
    for arg in args {
        let arg = arg.into();
        if arg == "--report-duplicate" {
            report_duplicate = true;
        } else if let Some(value) = arg
            .strip_prefix("-file-extension=")
            .or_else(|| arg.strip_prefix("--file-extension="))
        {
            if file_extensions.is_some() {
                return Err(CodeM8Error::new(
                    "file extensions were provided more than once",
                ));
            }
            file_extensions = Some(parse_file_extensions(value)?);
        } else if let Some(value) = arg
            .strip_prefix("-files=")
            .or_else(|| arg.strip_prefix("--files="))
        {
            if files.is_some() {
                return Err(CodeM8Error::new(
                    "explicit files were provided more than once",
                ));
            }
            files = Some(parse_file_list(value)?);
        } else {
            return Err(CodeM8Error::new(format!("unknown argument: {arg}")));
        }
    }
    if !report_duplicate {
        return Err(CodeM8Error::with_help(
            "no report switch provided; pass --report-duplicate",
        ));
    }
    Ok(CliConfig {
        report_duplicate,
        file_extensions: file_extensions.unwrap_or_else(supported_file_extensions),
        files,
    })
}

/// Parses a comma-separated list of file extensions.
///
/// # Errors
///
/// Returns an error when an extension is empty, starts with `.`, or contains a
/// path separator.
pub fn parse_file_extensions(value: &str) -> Result<Vec<String>> {
    let mut extensions = Vec::new();
    for raw_extension in value.split(',') {
        let extension = raw_extension.trim();
        if extension.is_empty() {
            return Err(CodeM8Error::new("file extension values must not be empty"));
        }
        if extension.starts_with('.') {
            return Err(CodeM8Error::new(format!(
                "file extensions must not start with a dot: {extension}"
            )));
        }
        if extension.contains('/') || extension.contains('\\') {
            return Err(CodeM8Error::new(format!(
                "file extensions must not contain path separators: {extension}"
            )));
        }
        let extension = extension.to_ascii_lowercase();
        if !extensions.contains(&extension) {
            extensions.push(extension);
        }
    }
    if extensions.is_empty() {
        return Err(CodeM8Error::new("at least one file extension is required"));
    }
    Ok(extensions)
}

/// Parses a comma-separated list of explicit file paths.
///
/// # Errors
///
/// Returns an error when any provided file path is empty.
pub fn parse_file_list(value: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for raw_file in value.split(',') {
        let file = raw_file.trim();
        if file.is_empty() {
            return Err(CodeM8Error::new("file path values must not be empty"));
        }
        files.push(PathBuf::from(file));
    }
    if files.is_empty() {
        return Err(CodeM8Error::new("at least one explicit file is required"));
    }
    Ok(files)
}

fn is_help_argument(arg: &str) -> bool {
    matches!(arg, "help" | "--help" | "-h")
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
    fn exposes_detailed_help_text() {
        assert!(help_text().contains("USAGE:"));
        assert!(help_text().contains("--report-duplicate"));
        assert!(help_text().contains("-file-extension=<extensions>"));
        assert!(help_text().contains("-files=<paths>"));
        assert!(help_text().contains("helps you find repeated code"));
        assert!(!help_text().contains("Duplicate weight"));
    }

    #[test]
    fn parses_default_duplicate_report_config() {
        let config = parse_args(["--report-duplicate"]).expect("config parses");
        assert!(config.report_duplicate);
        assert_eq!(config.file_extensions, supported_file_extensions());
        assert_eq!(config.files, None);
    }

    #[test]
    fn parses_extensions_case_insensitively_and_trims_whitespace() {
        let extensions = parse_file_extensions(" ts, JS ,tsx,ts ").expect("extensions parse");
        assert_eq!(extensions, ["ts", "js", "tsx"]);
    }

    #[test]
    fn rejects_empty_extensions() {
        let error = parse_file_extensions("ts,,js").expect_err("empty extension fails");
        assert!(error.to_string().contains("must not be empty"));
    }

    #[test]
    fn rejects_extensions_with_leading_dot() {
        let error = parse_file_extensions(".ts").expect_err("dot-prefixed extension fails");
        assert!(error.to_string().contains("must not start with a dot"));
    }

    #[test]
    fn rejects_extensions_with_path_separators() {
        let error = parse_file_extensions("src/ts").expect_err("path-like extension fails");
        assert!(error
            .to_string()
            .contains("must not contain path separators"));
    }

    #[test]
    fn rejects_missing_report_switch() {
        let error = parse_args(["-file-extension=rs"]).expect_err("missing report fails");
        assert!(error.to_string().contains("no report switch provided"));
        assert!(error.should_show_help());
    }

    #[test]
    fn rejects_unknown_arguments() {
        let error = parse_args(["--report-duplicate", "--verbose"]).expect_err("unknown arg fails");
        assert!(error.to_string().contains("unknown argument: --verbose"));
        assert!(!error.should_show_help());
    }

    #[test]
    fn rejects_repeated_file_extension_arguments() {
        let error = parse_args([
            "--report-duplicate",
            "-file-extension=ts",
            "--file-extension=js",
        ])
        .expect_err("repeated extensions fail");
        assert!(error
            .to_string()
            .contains("file extensions were provided more than once"));
    }

    #[test]
    fn rejects_repeated_explicit_file_arguments() {
        let error = parse_args(["--report-duplicate", "-files=a.ts", "--files=b.ts"])
            .expect_err("repeated explicit files fail");
        assert!(error
            .to_string()
            .contains("explicit files were provided more than once"));
    }

    #[test]
    fn parses_explicit_file_list() {
        let files = parse_file_list("src/a.ts, ./src/b.ts").expect("files parse");
        assert_eq!(
            files,
            [PathBuf::from("src/a.ts"), PathBuf::from("./src/b.ts")]
        );
    }

    #[test]
    fn rejects_empty_explicit_file_paths() {
        let error = parse_file_list("src/a.ts, ").expect_err("empty explicit file fails");
        assert!(error
            .to_string()
            .contains("file path values must not be empty"));
    }
}
