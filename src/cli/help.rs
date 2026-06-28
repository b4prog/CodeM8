use std::fmt::Write as _;

use super::version::codem8_version_from_cargo_lock;

const HELP_TEXT_BODY: &str = "\
USAGE:
  codem8 help
  codem8 -h
  codem8 --report-duplicate [OPTIONS]
  codem8 --report-complexity [OPTIONS]

COMMANDS:
  help
  -h
      Display this detailed documentation.

REQUIRED REPORT SWITCHES:
  --report-duplicate
      Analyze source files and print a duplicate code report.

  --report-complexity
      Analyze supported source files and print a function complexity report.
      Cannot be combined with --report-duplicate.

OPTIONS:
  -file-extension=<extensions>
      Comma-separated source file extensions to analyze.
      Defaults to all extensions registered in LANGUAGE_PATTERNS.
      Examples: -file-extension=ts,tsx,js,jsx

  -files=<paths>
      Comma-separated explicit files to analyze instead of recursively
      discovering files from the current directory.
      Example: -files=\"src/a.ts,src/b.js\"

  -git-branch
      Search only in files changed on the current local Git
      branch. Cannot be combined with -files.

  -git-branch-strict
      Search only in files changed on the current local Git branch, and report
      only duplicate blocks or complex functions that overlap changed lines.
      Cannot be combined with -files or -git-branch.

  -max-cognitive-complexity=<value>
      Maximum allowed cognitive complexity for --report-complexity.
      Defaults to 15.

  -max-cyclomatic-complexity=<value>
      Maximum allowed cyclomatic complexity for --report-complexity.
      Defaults to 10.

  -verbose
      Include analyzed files and timings in report output, plus duplicate block details.
      In -git-branch-strict mode, analyzed files include changed line ranges.

DUPLICATE REPORT PURPOSE:
  The duplicate report helps you find repeated code that may be worth
  refactoring, reviewing, or consolidating. It lists each duplicated block with
  the files and line ranges where it appears, making it easier to compare the
  repeated code and decide whether it should stay duplicated.

COMPLEXITY REPORT PURPOSE:
  The complexity report helps you find functions whose cognitive or cyclomatic
  complexity exceeds the configured limits. It lists each function with its
  location and both computed complexity values.

EXAMPLES:
  codem8 --report-duplicate
  codem8 --report-duplicate -file-extension=ts,tsx,js,jsx
  codem8 --report-duplicate -file-extension=ts,js -files=\"src/a.ts,src/b.js\"
  codem8 --report-duplicate -git-branch
  codem8 --report-duplicate -git-branch-strict
  codem8 --report-complexity -file-extension=rs -max-cognitive-complexity=12
";

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::version::codem8_version_from_cargo_lock;

    #[test]
    fn exposes_detailed_help_text() {
        let help = help_text();
        assert_help_includes_expected_sections(&help);
        assert_help_includes_single_dash_options(&help);
        assert_help_excludes_double_dash_options(&help);
    }

    fn assert_help_includes_expected_sections(help: &str) {
        assert!(help.contains("USAGE:"));
        assert!(help.contains("codem8 -h"));
        assert!(help.contains("  -h"));
        assert!(help.contains("--report-duplicate"));
        assert!(help.contains("--report-complexity"));
        assert!(help.contains("helps you find repeated code"));
        assert!(help.contains("helps you find functions"));
        assert!(!help.contains("Duplicate weight"));
    }

    fn assert_help_includes_single_dash_options(help: &str) {
        assert!(help.contains("-verbose"));
        assert!(help.contains("-file-extension=<extensions>"));
        assert!(help.contains("-files=<paths>"));
        assert!(help.contains("-git-branch"));
        assert!(help.contains("-git-branch-strict"));
        assert!(help.contains("-max-cognitive-complexity=<value>"));
        assert!(help.contains("-max-cyclomatic-complexity=<value>"));
    }

    fn assert_help_excludes_double_dash_options(help: &str) {
        assert!(!help.contains("--verbose"));
        assert!(!help.contains("--file-extension=<extensions>"));
        assert!(!help.contains("--files=<paths>"));
        assert!(!help.contains("--git-branch"));
        assert!(!help.contains("--git-branch-strict"));
        assert!(!help.contains("--max-cognitive-complexity=<value>"));
        assert!(!help.contains("--max-cyclomatic-complexity=<value>"));
    }

    #[test]
    fn help_text_includes_version_from_cargo_lock() {
        let version = codem8_version_from_cargo_lock().expect("codem8 version exists");
        assert!(help_text().starts_with(&format!("CodeM8 {version} - ")));
    }
}
