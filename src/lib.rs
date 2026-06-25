pub mod cli;
pub mod discovery;
pub mod duplicate;
pub mod error;
pub mod language;
pub mod line;
pub mod model;
pub mod paths;
pub mod report;

use std::io::Write;
use std::path::Path;

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
            let source_files = discovery::discover_source_files(
                current_dir,
                &config.file_extensions,
                config.files.as_deref(),
            )?;
            let processed_files = line::process_source_files(&source_files)?;
            let duplicate_blocks = duplicate::detect_duplicate_blocks(&processed_files);
            let report = report::DuplicateReport {
                analyzed_files: source_files.len(),
                analyzed_extensions: config.file_extensions,
                duplicate_blocks,
            };
            writer
                .write_all(report::render_duplicate_report(&report).as_bytes())
                .map_err(|error| {
                    CodeM8Error::new(format!("could not write report output: {error}"))
                })?;
        }
    }
    Ok(())
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
        assert_eq!(
            output,
            concat!(
                "Duplicate Code Report\n",
                "=====================\n",
                "\n",
                "Analyzed files: 2\n",
                "Analyzed extensions: ts\n",
                "Duplicate blocks found: 1\n",
                "\n",
                "#1 Weight: 324\n",
                "Lines: 4\n",
                "Characters: 81\n",
                "Occurrences: 2\n",
                "\n",
                "Locations:\n",
                "- src/a.ts:1-4\n",
                "- src/b.ts:1-4\n",
                "\n",
                "Code:\n",
                "  const value = computeValue(input);\n",
                "  if (value === undefined) {\n",
                "  return defaultValue;\n",
                "  }\n",
            )
        );
    }

    #[test]
    fn explicit_files_disable_recursive_discovery() {
        let project = TempProject::new("explicit-files");
        project.write("src/a.ts", "const value = one;\n");
        project.write("src/b.ts", "const value = one;\n");
        let output =
            run_in(&project, &["--report-duplicate", "-files=src/a.ts"]).expect("report succeeds");
        assert!(output.contains("Analyzed files: 1"));
        assert!(output.contains("Duplicate blocks found: 0"));
    }

    #[test]
    fn custom_extensions_change_analyzed_files() {
        let project = TempProject::new("custom-extensions");
        project.write("src/a.js", "const value = one;\n");
        project.write("src/b.js", "const value = one;\n");
        let default_output = run_in(&project, &["--report-duplicate"]).expect("report succeeds");
        assert!(default_output.contains("Analyzed files: 0"));
        let js_output = run_in(&project, &["--report-duplicate", "-file-extension=js"])
            .expect("report succeeds");
        assert!(js_output.contains("Analyzed files: 2"));
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
