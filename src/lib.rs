pub mod cli;
pub mod discovery;
pub mod duplicate;
pub mod error;
pub mod git;
pub mod language;
pub mod line;
pub mod model;
pub mod paths;
pub mod report;

use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::error::{CodeM8Error, Result};

/// Runs the CLI workflow and writes the selected report to the provided writer.
///
/// # Errors
///
/// Returns an error when argument parsing, file discovery, file processing, or
/// report writing fails.
pub fn run<I, S, W>(args: I, current_dir: &Path, writer: &mut W) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    W: Write,
{
    match cli::parse_command(args)? {
        cli::CliCommand::Help => writer
            .write_all(cli::help_text().as_bytes())
            .map_err(|error| CodeM8Error::new(format!("could not write help output: {error}")))?,
        cli::CliCommand::ReportDuplicate(config) => {
            let should_report_scanned_files = config.git_branch || config.files.is_some();
            let (source_files, discovery_duration) = time_result(config.verbose, || {
                let git_branch_files = if config.git_branch {
                    Some(git::changed_files_against_origin(current_dir)?)
                } else {
                    None
                };
                discovery::discover_source_files(
                    current_dir,
                    &config.file_extensions,
                    git_branch_files.as_deref().or(config.files.as_deref()),
                )
            })?;
            let (processed_files, file_processing_duration) =
                time_result(config.verbose, || line::process_source_files(&source_files))?;
            let (duplicate_blocks, duplicate_detection_duration) =
                time_value(config.verbose, || {
                    duplicate::detect_duplicate_blocks(&processed_files)
                });
            let report = report::DuplicateReport {
                analyzed_files: source_files.len(),
                analyzed_extensions: config.file_extensions,
                scanned_files: should_report_scanned_files.then(|| {
                    source_files
                        .iter()
                        .map(|source_file| source_file.display_path.clone())
                        .collect()
                }),
                timings: match (
                    discovery_duration,
                    file_processing_duration,
                    duplicate_detection_duration,
                ) {
                    (Some(discovery), Some(file_processing), Some(duplicate_detection)) => {
                        Some(report::DuplicateReportTimings {
                            discovery,
                            file_processing,
                            duplicate_detection,
                        })
                    }
                    _ => None,
                },
                duplicate_blocks,
            };
            writer
                .write_all(report::render_duplicate_report(&report, config.verbose).as_bytes())
                .map_err(|error| {
                    CodeM8Error::new(format!("could not write report output: {error}"))
                })?;
        }
    }
    Ok(())
}

fn time_result<T>(
    enabled: bool,
    operation: impl FnOnce() -> Result<T>,
) -> Result<(T, Option<Duration>)> {
    let started_at = enabled.then(Instant::now);
    let value = operation()?;
    Ok((value, started_at.map(|instant| instant.elapsed())))
}

fn time_value<T>(enabled: bool, operation: impl FnOnce() -> T) -> (T, Option<Duration>) {
    let started_at = enabled.then(Instant::now);
    let value = operation();
    (value, started_at.map(|instant| instant.elapsed()))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempProject {
        path: PathBuf,
    }

    impl TempProject {
        fn new(name: &str) -> Self {
            let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("codem8-{name}-{}-{id}", std::process::id()));
            if path.exists() {
                fs::remove_dir_all(&path).expect("remove stale test directory");
            }
            fs::create_dir_all(&path).expect("create test directory");
            Self { path }
        }

        fn write(&self, relative_path: &str, contents: &str) {
            let path = self.path.join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create test parent directory");
            }
            fs::write(path, contents).expect("write test file");
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn run_in(project: &TempProject, args: &[&str]) -> std::result::Result<String, CodeM8Error> {
        let mut output = Vec::new();
        run(args.iter().copied(), project.path(), &mut output)?;
        Ok(String::from_utf8(output).expect("report is UTF-8"))
    }

    #[test]
    fn duplicate_report_snapshot_is_stable() {
        let project = TempProject::new("snapshot");
        project.write(
            "src/a.ts",
            "const value = computeValue(input);\nif (value === undefined) {\nreturn defaultValue;\n}\n",
        );
        project.write(
            "src/b.ts",
            "const value = computeValue(input);\nif (value === undefined) {\nreturn defaultValue;\n}\n",
        );
        let output = run_in(&project, &["--report-duplicate"]).expect("report succeeds");
        let mut expected_extensions = language::supported_file_extensions();
        expected_extensions.sort();
        let expected_extensions = expected_extensions.join(", ");
        assert_eq!(
            output,
            [
                "Duplicate Code Report\n",
                "=====================\n",
                "\n",
                "Number of files scanned: 2\n",
                "Analyzed extensions: ",
                &expected_extensions,
                "\n",
                "Duplicate blocks found: 1\n",
                "\n",
                "#1\n",
                "- src/a.ts:1-4\n",
                "- src/b.ts:1-4\n",
            ]
            .concat()
        );
    }

    #[test]
    fn verbose_duplicate_report_includes_metrics_without_characters() {
        let project = TempProject::new("verbose");
        project.write(
            "src/a.ts",
            "const value = computeValue(input);\nreturn value;\n",
        );
        project.write(
            "src/b.ts",
            "const value = computeValue(input);\nreturn value;\n",
        );
        let output =
            run_in(&project, &["--report-duplicate", "-verbose"]).expect("report succeeds");
        assert!(output.contains("Weight:"));
        assert!(output.contains("Lines: 2"));
        assert!(output.contains("Occurrences: 2"));
        assert!(output.contains("Timings:"));
        assert!(output.contains("- Discovery:"));
        assert!(output.contains("- File processing:"));
        assert!(output.contains("- Duplicate detection:"));
        assert!(!output.contains("Characters:"));
        assert!(
            output.find("Code:").expect("code section exists")
                < output.find("Locations:").expect("locations section exists")
        );
    }

    #[test]
    fn explicit_files_disable_recursive_discovery() {
        let project = TempProject::new("explicit-files");
        project.write("src/a.ts", "const value = one;\n");
        project.write("src/b.ts", "const value = one;\n");
        let output =
            run_in(&project, &["--report-duplicate", "-files=src/a.ts"]).expect("report succeeds");
        assert!(output.contains("Number of files scanned: 1"));
        assert!(output.contains("Duplicate blocks found: 0"));
    }

    #[test]
    fn verbose_explicit_files_report_lists_scanned_files() {
        let project = TempProject::new("verbose-explicit-files");
        project.write("src/a.ts", "const value = one;\n");
        project.write("src/b.ts", "const value = one;\n");
        let quiet_output =
            run_in(&project, &["--report-duplicate", "-files=src/a.ts"]).expect("report succeeds");
        assert!(!quiet_output.contains("Files scanned:"));
        let verbose_output = run_in(
            &project,
            &["--report-duplicate", "-verbose", "-files=src/a.ts"],
        )
        .expect("report succeeds");
        assert!(verbose_output.contains(
            "Number of files scanned: 1\n\
             Files scanned:\n\
             - src/a.ts\n\
             Analyzed extensions:"
        ));
    }

    #[test]
    fn custom_extensions_change_analyzed_files() {
        let project = TempProject::new("custom-extensions");
        project.write("src/a.js", "const value = one;\n");
        project.write("src/b.js", "const value = one;\n");
        let default_output = run_in(&project, &["--report-duplicate"]).expect("report succeeds");
        assert!(default_output.contains("Number of files scanned: 2"));
        assert!(default_output.contains("Duplicate blocks found: 1"));
        let js_output = run_in(&project, &["--report-duplicate", "-file-extension=js"])
            .expect("report succeeds");
        assert!(js_output.contains("Number of files scanned: 2"));
        assert!(js_output.contains("Duplicate blocks found: 1"));
    }

    #[test]
    fn invalid_explicit_file_returns_a_clear_error() {
        let project = TempProject::new("invalid-file");
        let error = run_in(&project, &["--report-duplicate", "-files=missing.ts"])
            .expect_err("missing explicit file fails");
        assert!(error
            .to_string()
            .contains("explicit file does not exist: missing.ts"));
    }

    #[test]
    fn help_command_prints_documentation() {
        let project = TempProject::new("help");
        let output = run_in(&project, &["help"]).expect("help succeeds");
        assert!(output.contains("USAGE:"));
        assert!(output.contains("--report-duplicate"));
    }
}
