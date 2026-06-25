use std::fmt::Write as _;
use std::path::PathBuf;

use crate::error::{CodeM8Error, Result};
use crate::language::supported_file_extensions;

const CARGO_LOCK: &str = include_str!("../Cargo.lock");
const HELP_TEXT_BODY: &str = "\
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
      Comma-separated source file extensions to analyze.
      Defaults to all extensions registered in LANGUAGE_PATTERNS.
      Examples: -file-extension=ts,tsx,js,jsx

  -files=<paths>
      Comma-separated explicit files to analyze instead of recursively
      discovering files from the current directory.
      Example: -files=src/a.ts,src/b.js

  -git-branch
      Analyze files changed on the current local Git branch compared to the
      origin base branch, including committed, staged, unstaged, and untracked
      files. Cannot be combined with -files.

  -verbose
      Include duplicate block metrics in report output.

DUPLICATE REPORT PURPOSE:
  The duplicate report helps you find repeated code that may be worth
  refactoring, reviewing, or consolidating. It lists each duplicated block with
  the files and line ranges where it appears, making it easier to compare the
  repeated code and decide whether it should stay duplicated.

EXAMPLES:
  codem8 --report-duplicate
  codem8 --report-duplicate -file-extension=ts,tsx,js,jsx
  codem8 --report-duplicate -file-extension=ts,js -files=src/a.ts,src/b.js
  codem8 --report-duplicate -git-branch
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CargoLockPackage<'a> {
    name: &'a str,
    version: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Help,
    ReportDuplicate(CliConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliConfig {
    pub report_duplicate: bool,
    pub verbose: bool,
    pub file_extensions: Vec<String>,
    pub files: Option<Vec<PathBuf>>,
    pub git_branch: bool,
}

#[must_use]
pub fn help_text() -> String {
    let version = codem8_version_from_cargo_lock().unwrap_or("unknown");
    let mut output = String::new();
    let _ = writeln!(
        output,
        "CodeM8 {version} - deterministic source code analysis reports."
    );
    output.push('\n');
    output.push_str(HELP_TEXT_BODY);
    output
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
    let mut verbose = false;
    let mut file_extensions = None;
    let mut files = None;
    let mut git_branch = false;
    for arg in args {
        let arg = arg.into();
        if arg == "--report-duplicate" {
            report_duplicate = true;
        } else if arg == "-verbose" {
            verbose = true;
        } else if arg == "-git-branch" {
            if git_branch {
                return Err(CodeM8Error::new(
                    "git branch mode was provided more than once",
                ));
            }
            git_branch = true;
        } else if let Some(value) = arg.strip_prefix("-file-extension=") {
            if file_extensions.is_some() {
                return Err(CodeM8Error::new(
                    "file extensions were provided more than once",
                ));
            }
            file_extensions = Some(parse_file_extensions(value)?);
        } else if let Some(value) = arg.strip_prefix("-files=") {
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
    if git_branch && files.is_some() {
        return Err(CodeM8Error::new(
            "git branch mode cannot be combined with explicit files",
        ));
    }
    Ok(CliConfig {
        report_duplicate,
        verbose,
        file_extensions: file_extensions.unwrap_or_else(supported_file_extensions),
        files,
        git_branch,
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
    matches!(arg, "help" | "-h")
}

fn codem8_version_from_cargo_lock() -> Option<&'static str> {
    cargo_lock_packages(CARGO_LOCK)
        .find(|package| package.name == "codem8")
        .map(|package| package.version)
}

fn cargo_lock_packages(lockfile: &str) -> impl Iterator<Item = CargoLockPackage<'_>> {
    lockfile.split("[[package]]").filter_map(cargo_lock_package)
}

fn cargo_lock_package(section: &str) -> Option<CargoLockPackage<'_>> {
    let name = cargo_lock_value(section, "name")?;
    let version = cargo_lock_value(section, "version")?;
    Some(CargoLockPackage { name, version })
}

fn cargo_lock_value<'a>(section: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key} = \"");
    section
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(&prefix)?.strip_suffix('"'))
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
    fn exposes_detailed_help_text() {
        let help = help_text();
        assert!(help.contains("USAGE:"));
        assert!(help.contains("--report-duplicate"));
        assert!(help.contains("-verbose"));
        assert!(help.contains("-file-extension=<extensions>"));
        assert!(help.contains("-files=<paths>"));
        assert!(help.contains("-git-branch"));
        assert!(!help.contains("--verbose"));
        assert!(!help.contains("--file-extension=<extensions>"));
        assert!(!help.contains("--files=<paths>"));
        assert!(!help.contains("--git-branch"));
        assert!(help.contains("helps you find repeated code"));
        assert!(!help.contains("Duplicate weight"));
    }

    #[test]
    fn help_text_includes_version_from_cargo_lock() {
        let version = codem8_version_from_cargo_lock().expect("codem8 version exists");
        assert!(help_text().starts_with(&format!("CodeM8 {version} - ")));
    }

    #[test]
    fn extracts_package_versions_from_cargo_lock_sections() {
        let lockfile = r#"
[[package]]
name = "dependency"
version = "1.2.3"

[[package]]
name = "codem8"
version = "0.4.2"
"#;
        let package = cargo_lock_packages(lockfile)
            .find(|package| package.name == "codem8")
            .expect("package exists");
        assert_eq!(package.version, "0.4.2");
    }

    #[test]
    fn parses_default_duplicate_report_config() {
        let config = parse_args(["--report-duplicate"]).expect("config parses");
        assert!(config.report_duplicate);
        assert!(!config.verbose);
        assert_eq!(config.file_extensions, supported_file_extensions());
        assert_eq!(config.files, None);
        assert!(!config.git_branch);
    }

    #[test]
    fn parses_verbose_duplicate_report_config() {
        let config = parse_args(["--report-duplicate", "-verbose"]).expect("config parses");
        assert!(config.report_duplicate);
        assert!(config.verbose);
    }

    #[test]
    fn parses_git_branch_duplicate_report_config() {
        let config = parse_args(["--report-duplicate", "-git-branch"]).expect("config parses");
        assert!(config.git_branch);
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
        let error = parse_args(["--report-duplicate", "--unknown"]).expect_err("unknown arg fails");
        assert!(error.to_string().contains("unknown argument: --unknown"));
        assert!(!error.should_show_help());
    }

    #[test]
    fn rejects_double_dash_option_arguments() {
        for option in [
            "--help",
            "--verbose",
            "--file-extension=js",
            "--files=src/a.ts",
            "--git-branch",
        ] {
            let error =
                parse_args(["--report-duplicate", option]).expect_err("double-dash option fails");
            assert!(error
                .to_string()
                .contains(&format!("unknown argument: {option}")));
        }
    }

    #[test]
    fn rejects_repeated_file_extension_arguments() {
        let error = parse_args([
            "--report-duplicate",
            "-file-extension=ts",
            "-file-extension=js",
        ])
        .expect_err("repeated extensions fail");
        assert!(error
            .to_string()
            .contains("file extensions were provided more than once"));
    }

    #[test]
    fn rejects_repeated_explicit_file_arguments() {
        let error = parse_args(["--report-duplicate", "-files=a.ts", "-files=b.ts"])
            .expect_err("repeated explicit files fail");
        assert!(error
            .to_string()
            .contains("explicit files were provided more than once"));
    }

    #[test]
    fn rejects_repeated_git_branch_arguments() {
        let error = parse_args(["--report-duplicate", "-git-branch", "-git-branch"])
            .expect_err("repeated git branch mode fails");
        assert!(error
            .to_string()
            .contains("git branch mode was provided more than once"));
    }

    #[test]
    fn rejects_git_branch_with_explicit_files() {
        let error = parse_args(["--report-duplicate", "-git-branch", "-files=a.ts"])
            .expect_err("exclusive file modes fail");
        assert!(error
            .to_string()
            .contains("git branch mode cannot be combined with explicit files"));
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
