use std::path::PathBuf;

use clap::{ArgAction, Parser};

use super::{CliConfig, ReportKind};
use crate::error::{CodeM8Error, Result};
use crate::language::supported_file_extensions;

pub const DEFAULT_MAX_COGNITIVE_COMPLEXITY: u32 = 15;
pub const DEFAULT_MAX_CYCLOMATIC_COMPLEXITY: u32 = 10;

#[derive(Debug, Parser)]
#[command(name = "codem8", disable_help_flag = true, disable_version_flag = true)]
struct ClapCli {
    #[arg(long = "report-duplicate", action = ArgAction::Count)]
    report_duplicate: u8,
    #[arg(long = "report-complexity", action = ArgAction::Count)]
    report_complexity: u8,
    #[arg(long = "codem8-verbose", action = ArgAction::Count)]
    verbose: u8,
    #[arg(long = "codem8-git-branch", action = ArgAction::Count)]
    git_branch: u8,
    #[arg(
        long = "codem8-file-extension",
        value_name = "extensions",
        value_parser = parse_file_extensions,
        action = ArgAction::Append
    )]
    file_extensions: Vec<Vec<String>>,
    #[arg(
        long = "codem8-files",
        value_name = "paths",
        value_parser = parse_file_list,
        action = ArgAction::Append
    )]
    files: Vec<Vec<PathBuf>>,
    #[arg(
        long = "codem8-max-cognitive-complexity",
        value_parser = parse_complexity_limit
    )]
    max_cognitive_complexity: Option<u32>,
    #[arg(
        long = "codem8-max-cyclomatic-complexity",
        value_parser = parse_complexity_limit
    )]
    max_cyclomatic_complexity: Option<u32>,
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
    let parsed = ClapCli::try_parse_from(normalized_clap_args(args)?)
        .map_err(|error| CodeM8Error::new(error.to_string().trim().to_owned()))?;
    let report = selected_report(&parsed)?;
    validate_repeated_options(&parsed)?;
    let git_branch = parsed.git_branch != 0;
    let files = selected_files(&parsed, git_branch)?;
    validate_complexity_limits(report, &parsed)?;
    Ok(CliConfig {
        report,
        verbose: parsed.verbose != 0,
        file_extensions: selected_file_extensions(&parsed),
        files,
        git_branch,
        max_cognitive_complexity: parsed
            .max_cognitive_complexity
            .unwrap_or(DEFAULT_MAX_COGNITIVE_COMPLEXITY),
        max_cyclomatic_complexity: parsed
            .max_cyclomatic_complexity
            .unwrap_or(DEFAULT_MAX_CYCLOMATIC_COMPLEXITY),
    })
}

fn selected_report(parsed: &ClapCli) -> Result<ReportKind> {
    let report_count = parsed.report_duplicate + parsed.report_complexity;
    if report_count == 0 {
        return Err(CodeM8Error::with_help(
            "no report switch provided; pass --report-duplicate or --report-complexity",
        ));
    }
    if parsed.report_duplicate > 1 || parsed.report_complexity > 1 {
        return Err(CodeM8Error::new(
            "report switch was provided more than once",
        ));
    }
    if report_count > 1 {
        return Err(CodeM8Error::new(
            "--report-duplicate and --report-complexity are mutually exclusive",
        ));
    }
    Ok(if parsed.report_duplicate != 0 {
        ReportKind::Duplicate
    } else {
        ReportKind::Complexity
    })
}

fn validate_repeated_options(parsed: &ClapCli) -> Result<()> {
    if parsed.git_branch > 1 {
        return Err(CodeM8Error::new(
            "git branch mode was provided more than once",
        ));
    }
    if parsed.file_extensions.len() > 1 {
        return Err(CodeM8Error::new(
            "file extensions were provided more than once",
        ));
    }
    if parsed.files.len() > 1 {
        return Err(CodeM8Error::new(
            "explicit files were provided more than once",
        ));
    }
    Ok(())
}

fn selected_files(parsed: &ClapCli, git_branch: bool) -> Result<Option<Vec<PathBuf>>> {
    let files = parsed.files.first().cloned();
    if git_branch && files.is_some() {
        return Err(CodeM8Error::new(
            "git branch mode cannot be combined with explicit files",
        ));
    }
    Ok(files)
}

fn validate_complexity_limits(report: ReportKind, parsed: &ClapCli) -> Result<()> {
    if report == ReportKind::Duplicate
        && (parsed.max_cognitive_complexity.is_some() || parsed.max_cyclomatic_complexity.is_some())
    {
        return Err(CodeM8Error::new(
            "complexity limits can only be used with --report-complexity",
        ));
    }
    Ok(())
}

fn selected_file_extensions(parsed: &ClapCli) -> Vec<String> {
    parsed
        .file_extensions
        .first()
        .cloned()
        .unwrap_or_else(supported_file_extensions)
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

/// Parses a positive complexity limit.
///
/// # Errors
///
/// Returns an error when the value is not a positive integer.
pub fn parse_complexity_limit(value: &str) -> Result<u32> {
    let limit = value.parse::<u32>().map_err(|_| {
        CodeM8Error::new(format!(
            "complexity limits must be positive integers: {value}"
        ))
    })?;
    if limit == 0 {
        return Err(CodeM8Error::new(
            "complexity limits must be greater than zero",
        ));
    }
    Ok(limit)
}

fn normalized_clap_args<I, S>(args: I) -> Result<Vec<String>>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut normalized = vec!["codem8".to_owned()];
    for arg in join_split_file_extensions(args.into_iter().map(Into::into)) {
        normalized.push(normalized_clap_arg(arg)?);
    }
    Ok(normalized)
}

fn join_split_file_extensions(args: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut joined = Vec::new();
    for arg in args {
        if should_join_split_extension(joined.last(), &arg) {
            let previous = joined
                .last_mut()
                .expect("previous file argument exists when extension joins");
            previous.push_str(&arg);
        } else {
            joined.push(arg);
        }
    }
    joined
}

fn should_join_split_extension(previous: Option<&String>, arg: &str) -> bool {
    previous.is_some_and(|previous| previous.starts_with("-files=") && arg.starts_with('.'))
}

fn normalized_clap_arg(arg: String) -> Result<String> {
    if arg == "-verbose" {
        Ok("--codem8-verbose".to_owned())
    } else if arg == "-git-branch" {
        Ok("--codem8-git-branch".to_owned())
    } else if let Some(value) = arg.strip_prefix("-file-extension=") {
        Ok(format!("--codem8-file-extension={value}"))
    } else if let Some(value) = arg.strip_prefix("-files=") {
        Ok(format!("--codem8-files={value}"))
    } else if let Some(value) = arg.strip_prefix("-max-cognitive-complexity=") {
        Ok(format!("--codem8-max-cognitive-complexity={value}"))
    } else if let Some(value) = arg.strip_prefix("-max-cyclomatic-complexity=") {
        Ok(format!("--codem8-max-cyclomatic-complexity={value}"))
    } else if arg.starts_with("--")
        && !matches!(arg.as_str(), "--report-duplicate" | "--report-complexity")
    {
        Err(CodeM8Error::new(format!("unknown argument: {arg}")))
    } else {
        Ok(arg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_duplicate_report_config() {
        let config = parse_args(["--report-duplicate"]).expect("config parses");
        assert_eq!(config.report, ReportKind::Duplicate);
        assert!(!config.verbose);
        assert_eq!(config.file_extensions, supported_file_extensions());
        assert_eq!(config.files, None);
        assert!(!config.git_branch);
        assert_eq!(
            config.max_cognitive_complexity,
            DEFAULT_MAX_COGNITIVE_COMPLEXITY
        );
        assert_eq!(
            config.max_cyclomatic_complexity,
            DEFAULT_MAX_CYCLOMATIC_COMPLEXITY
        );
    }

    #[test]
    fn parses_default_complexity_report_config() {
        let config = parse_args(["--report-complexity"]).expect("config parses");
        assert_eq!(config.report, ReportKind::Complexity);
        assert_eq!(
            config.max_cognitive_complexity,
            DEFAULT_MAX_COGNITIVE_COMPLEXITY
        );
        assert_eq!(
            config.max_cyclomatic_complexity,
            DEFAULT_MAX_CYCLOMATIC_COMPLEXITY
        );
    }

    #[test]
    fn parses_custom_complexity_limits() {
        let config = parse_args([
            "--report-complexity",
            "-max-cognitive-complexity=20",
            "-max-cyclomatic-complexity=12",
        ])
        .expect("config parses");
        assert_eq!(config.max_cognitive_complexity, 20);
        assert_eq!(config.max_cyclomatic_complexity, 12);
    }

    #[test]
    fn parses_verbose_duplicate_report_config() {
        let config = parse_args(["--report-duplicate", "-verbose"]).expect("config parses");
        assert_eq!(config.report, ReportKind::Duplicate);
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
            "--max-cognitive-complexity=20",
            "--max-cyclomatic-complexity=12",
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
    fn rejects_repeated_report_switches() {
        let error = parse_args(["--report-duplicate", "--report-duplicate"])
            .expect_err("repeated report switch fails");
        assert!(error
            .to_string()
            .contains("report switch was provided more than once"));
    }

    #[test]
    fn rejects_multiple_report_kinds() {
        let error = parse_args(["--report-duplicate", "--report-complexity"])
            .expect_err("exclusive reports fail");
        assert!(error.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn rejects_zero_complexity_limits() {
        let error = parse_args(["--report-complexity", "-max-cognitive-complexity=0"])
            .expect_err("zero limit fails");
        assert!(error.to_string().contains("greater than zero"));
    }

    #[test]
    fn rejects_complexity_limits_with_duplicate_report() {
        let error = parse_args(["--report-duplicate", "-max-cognitive-complexity=15"])
            .expect_err("duplicate report complexity limit fails");
        assert!(error
            .to_string()
            .contains("can only be used with --report-complexity"));
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
    fn rejects_removed_git_branch_strict_argument() {
        let error = parse_args(["--report-duplicate", "-git-branch-strict"])
            .expect_err("removed git branch mode fails");
        assert!(error.to_string().contains("unexpected argument"));
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
    fn rejoins_powershell_split_file_extensions() {
        let config =
            parse_args(["--report-complexity", "-files=src/main", ".rs"]).expect("config parses");
        assert_eq!(config.files, Some(vec![PathBuf::from("src/main.rs")]));
    }

    #[test]
    fn rejoins_multiple_powershell_split_file_extensions() {
        let config = parse_args([
            "--report-complexity",
            "-files=src/main",
            ".rs,src/lib",
            ".rs",
        ])
        .expect("config parses");
        assert_eq!(
            config.files,
            Some(vec![
                PathBuf::from("src/main.rs"),
                PathBuf::from("src/lib.rs")
            ])
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
