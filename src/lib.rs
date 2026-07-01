#![allow(clippy::multiple_crate_versions)]

pub mod cli;
pub mod discovery;
pub mod error;
pub mod language;
pub mod line;
pub mod model;
pub mod paths;
pub mod report;

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::error::{CodeM8Error, Result};
use crate::model::SourceFile;
use crate::model::{
    AnalyzedFile, ChangedFileLines, DuplicateBlock, DuplicateOccurrence, FunctionComplexity,
    LineRange, ProcessedFile,
};
use crate::paths::format_path;

struct BranchScope {
    files: Option<Vec<PathBuf>>,
    lines: Option<Vec<ChangedFileLines>>,
    strict_file_paths: Option<Vec<PathBuf>>,
}

impl BranchScope {
    fn files(&self) -> Option<&[PathBuf]> {
        self.files.as_deref().or(self.strict_file_paths.as_deref())
    }

    fn lines(&self) -> Option<&[ChangedFileLines]> {
        self.lines.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Success,
    IssuesFound,
}

impl RunStatus {
    const fn from_issue_count(issue_count: usize) -> Self {
        if issue_count == 0 {
            Self::Success
        } else {
            Self::IssuesFound
        }
    }

    #[must_use]
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }
}

/// Runs the CLI workflow and writes the selected report to the provided writer.
///
/// # Errors
///
/// Returns an error when argument parsing, file discovery, file processing, or
/// report writing fails.
pub fn run<I, S, W>(args: I, current_dir: &Path, writer: &mut W) -> Result<RunStatus>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    W: Write,
{
    let status = match cli::parse_command(args)? {
        cli::CliCommand::Help => {
            write_help(writer)?;
            RunStatus::Success
        }
        cli::CliCommand::Version => {
            write_version(writer)?;
            RunStatus::Success
        }
        cli::CliCommand::Report(config) => match config.report {
            cli::ReportKind::Duplicate => run_duplicate_report(&config, current_dir, writer)?,
            cli::ReportKind::Complexity => run_complexity_report(&config, current_dir, writer)?,
        },
    };
    Ok(status)
}

fn write_help<W: Write>(writer: &mut W) -> Result<()> {
    writer
        .write_all(cli::help_text().as_bytes())
        .map_err(|error| CodeM8Error::new(format!("could not write help output: {error}")))
}

fn write_version<W: Write>(writer: &mut W) -> Result<()> {
    let version = cli::codem8_version_from_cargo_lock().unwrap_or("unknown");
    writeln!(writer, "{version}")
        .map_err(|error| CodeM8Error::new(format!("could not write version output: {error}")))
}

fn run_duplicate_report<W: Write>(
    config: &cli::CliConfig,
    current_dir: &Path,
    writer: &mut W,
) -> Result<RunStatus> {
    let branch_scope = changed_branch_scope(config, current_dir)?;
    let (source_files, discovery_duration) = discover_report_files(
        config.verbose,
        current_dir,
        &config.file_extensions,
        duplicate_discovery_files(config),
    )?;
    let (processed_files, file_processing_duration) =
        time_result(config.verbose, || line::process_source_files(&source_files))?;
    let analyzed_source_files = filtered_processed_files_for_scope(&processed_files, &branch_scope);
    let (duplicate_blocks, duplicate_detection_duration) = time_value(config.verbose, || {
        report::detect_duplicate_blocks(&processed_files)
    });
    let duplicate_blocks = filtered_duplicate_blocks_for_scope(duplicate_blocks, &branch_scope);
    let report = report::DuplicateReport {
        analyzed_files: analyzed_source_files.len(),
        analyzed_extensions: config.file_extensions.clone(),
        analyzed_file_paths: config
            .verbose
            .then(|| analyzed_processed_files(&analyzed_source_files, branch_scope.lines())),
        timings: duplicate_timings(
            discovery_duration,
            file_processing_duration,
            duplicate_detection_duration,
        ),
        duplicate_blocks,
    };
    let output = report::render_duplicate_report(&report, config.verbose);
    let status = RunStatus::from_issue_count(report.duplicate_blocks.len());
    write_report_output(writer, &output)?;
    Ok(status)
}

fn run_complexity_report<W: Write>(
    config: &cli::CliConfig,
    current_dir: &Path,
    writer: &mut W,
) -> Result<RunStatus> {
    let branch_scope = changed_branch_scope(config, current_dir)?;
    let analyzed_extensions = report::complexity_supported_file_extensions(&config.file_extensions);
    let (complexity_source_files, discovery_duration) = discover_report_files(
        config.verbose,
        current_dir,
        &analyzed_extensions,
        branch_scope.files().or(config.files.as_deref()),
    )?;
    let (functions, complexity_analysis_duration) = time_result(config.verbose, || {
        report::detect_complex_functions(
            &complexity_source_files,
            config.max_cognitive_complexity,
            config.max_cyclomatic_complexity,
        )
    })?;
    let functions = match branch_scope.lines() {
        Some(git_branch_lines) => filtered_strict_complex_functions(functions, git_branch_lines),
        None => functions,
    };
    let report = report::ComplexityReport {
        analyzed_files: complexity_source_files.len(),
        analyzed_extensions,
        analyzed_file_paths: config
            .verbose
            .then(|| analyzed_source_file_paths(&complexity_source_files, branch_scope.lines())),
        max_cognitive_complexity: config.max_cognitive_complexity,
        max_cyclomatic_complexity: config.max_cyclomatic_complexity,
        timings: complexity_timings(discovery_duration, complexity_analysis_duration),
        functions,
    };
    let output = report::render_complexity_report(&report, config.verbose);
    let status = RunStatus::from_issue_count(report.functions.len());
    write_report_output(writer, &output)?;
    Ok(status)
}

fn changed_branch_scope(config: &cli::CliConfig, current_dir: &Path) -> Result<BranchScope> {
    let files = changed_git_branch_files(config, current_dir)?;
    let lines = changed_git_branch_lines(config, current_dir)?;
    let strict_file_paths = lines.as_ref().map(|lines| changed_line_paths(lines));
    Ok(BranchScope {
        files,
        lines,
        strict_file_paths,
    })
}

fn changed_git_branch_files(
    config: &cli::CliConfig,
    current_dir: &Path,
) -> Result<Option<Vec<PathBuf>>> {
    if config.git_branch {
        discovery::changed_files_against_origin(current_dir).map(Some)
    } else {
        Ok(None)
    }
}

fn changed_git_branch_lines(
    config: &cli::CliConfig,
    current_dir: &Path,
) -> Result<Option<Vec<ChangedFileLines>>> {
    if config.git_branch_strict {
        discovery::changed_lines_against_origin(current_dir).map(Some)
    } else {
        Ok(None)
    }
}

fn duplicate_discovery_files(config: &cli::CliConfig) -> Option<&[PathBuf]> {
    if config.git_branch || config.git_branch_strict {
        None
    } else {
        config.files.as_deref()
    }
}

fn discover_report_files(
    verbose: bool,
    current_dir: &Path,
    file_extensions: &[String],
    files: Option<&[std::path::PathBuf]>,
) -> Result<(Vec<SourceFile>, Option<Duration>)> {
    time_result(verbose, || {
        discovery::discover_source_files(current_dir, file_extensions, files)
    })
}

const fn duplicate_timings(
    discovery: Option<Duration>,
    file_processing: Option<Duration>,
    duplicate_detection: Option<Duration>,
) -> Option<report::DuplicateReportTimings> {
    match (discovery, file_processing, duplicate_detection) {
        (Some(discovery), Some(file_processing), Some(duplicate_detection)) => {
            Some(report::DuplicateReportTimings {
                discovery,
                file_processing,
                duplicate_detection,
            })
        }
        _ => None,
    }
}

const fn complexity_timings(
    discovery: Option<Duration>,
    complexity_analysis: Option<Duration>,
) -> Option<report::ComplexityReportTimings> {
    match (discovery, complexity_analysis) {
        (Some(discovery), Some(complexity_analysis)) => Some(report::ComplexityReportTimings {
            discovery,
            complexity_analysis,
        }),
        _ => None,
    }
}

fn write_report_output<W: Write>(writer: &mut W, output: &str) -> Result<()> {
    writer
        .write_all(output.as_bytes())
        .map_err(|error| CodeM8Error::new(format!("could not write report output: {error}")))
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

fn filtered_processed_files(
    processed_files: &[ProcessedFile],
    selected_files: &[PathBuf],
) -> Vec<ProcessedFile> {
    let selected_files = selected_files
        .iter()
        .map(|path| format_path(path))
        .collect::<HashSet<_>>();
    processed_files
        .iter()
        .filter(|processed_file| {
            selected_files.contains(&format_path(&processed_file.source.display_path))
        })
        .cloned()
        .collect()
}

fn filtered_processed_files_for_scope(
    processed_files: &[ProcessedFile],
    branch_scope: &BranchScope,
) -> Vec<ProcessedFile> {
    branch_scope.files().map_or_else(
        || processed_files.to_vec(),
        |files| filtered_processed_files(processed_files, files),
    )
}

fn filtered_duplicate_blocks(
    duplicate_blocks: Vec<DuplicateBlock>,
    selected_files: &[PathBuf],
) -> Vec<DuplicateBlock> {
    let selected_files = selected_files
        .iter()
        .map(|path| format_path(path))
        .collect::<HashSet<_>>();
    duplicate_blocks
        .into_iter()
        .filter(|duplicate_block| {
            duplicate_block
                .occurrences
                .iter()
                .any(|occurrence| selected_files.contains(&format_path(&occurrence.file_path)))
        })
        .collect()
}

fn filtered_duplicate_blocks_for_scope(
    duplicate_blocks: Vec<DuplicateBlock>,
    branch_scope: &BranchScope,
) -> Vec<DuplicateBlock> {
    let duplicate_blocks = match branch_scope.files() {
        Some(files) => filtered_duplicate_blocks(duplicate_blocks, files),
        None => duplicate_blocks,
    };
    match branch_scope.lines() {
        Some(lines) => filtered_strict_duplicate_blocks(duplicate_blocks, lines),
        None => duplicate_blocks,
    }
}

fn changed_line_paths(changed_lines: &[ChangedFileLines]) -> Vec<PathBuf> {
    changed_lines
        .iter()
        .map(|changed_file| changed_file.path.clone())
        .collect()
}

fn analyzed_processed_files(
    processed_files: &[ProcessedFile],
    changed_lines: Option<&[ChangedFileLines]>,
) -> Vec<AnalyzedFile> {
    processed_files
        .iter()
        .map(|processed_file| analyzed_file(&processed_file.source.display_path, changed_lines))
        .collect()
}

fn analyzed_source_file_paths(
    source_files: &[SourceFile],
    changed_lines: Option<&[ChangedFileLines]>,
) -> Vec<AnalyzedFile> {
    source_files
        .iter()
        .map(|source_file| analyzed_file(&source_file.display_path, changed_lines))
        .collect()
}

fn analyzed_file(path: &Path, changed_lines: Option<&[ChangedFileLines]>) -> AnalyzedFile {
    AnalyzedFile {
        path: path.to_path_buf(),
        changed_lines: changed_lines
            .and_then(|changed_lines| changed_lines_for_path(path, changed_lines)),
    }
}

fn changed_lines_for_path(
    path: &Path,
    changed_lines: &[ChangedFileLines],
) -> Option<Vec<LineRange>> {
    let formatted_path = format_path(path);
    changed_lines
        .iter()
        .find(|changed_file| format_path(&changed_file.path) == formatted_path)
        .map(|changed_file| changed_file.lines.clone())
}

fn filtered_strict_duplicate_blocks(
    duplicate_blocks: Vec<DuplicateBlock>,
    changed_lines: &[ChangedFileLines],
) -> Vec<DuplicateBlock> {
    duplicate_blocks
        .into_iter()
        .filter(|duplicate_block| {
            duplicate_block_applies_to_changed_lines(duplicate_block, changed_lines)
        })
        .collect()
}

fn duplicate_block_applies_to_changed_lines(
    duplicate_block: &DuplicateBlock,
    changed_lines: &[ChangedFileLines],
) -> bool {
    duplicate_block
        .occurrences
        .iter()
        .any(|occurrence| occurrence_applies_to_changed_lines(occurrence, changed_lines))
}

fn occurrence_applies_to_changed_lines(
    occurrence: &DuplicateOccurrence,
    changed_lines: &[ChangedFileLines],
) -> bool {
    changed_lines_for_path(&occurrence.file_path, changed_lines).is_some_and(|lines| {
        ranges_overlap_lines(occurrence.start_line, occurrence.end_line, &lines)
    })
}

fn filtered_strict_complex_functions(
    functions: Vec<FunctionComplexity>,
    changed_lines: &[ChangedFileLines],
) -> Vec<FunctionComplexity> {
    functions
        .into_iter()
        .filter(|function| function_applies_to_changed_lines(function, changed_lines))
        .collect()
}

fn function_applies_to_changed_lines(
    function: &FunctionComplexity,
    changed_lines: &[ChangedFileLines],
) -> bool {
    changed_lines_for_path(&function.file_path, changed_lines)
        .is_some_and(|lines| ranges_overlap_lines(function.start_line, function.end_line, &lines))
}

fn ranges_overlap_lines(start: usize, end: usize, lines: &[LineRange]) -> bool {
    lines
        .iter()
        .any(|line_range| start <= line_range.end && end >= line_range.start)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
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
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    impl AsRef<Path> for TempProject {
        fn as_ref(&self) -> &Path {
            &self.path
        }
    }

    struct TempGitRepo {
        path: PathBuf,
    }

    impl TempGitRepo {
        fn new(name: &str) -> Self {
            let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("codem8-git-{name}-{}-{id}", std::process::id()));
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

        fn git(&self, args: &[&str]) {
            let status = Command::new("git")
                .arg("-C")
                .arg(&self.path)
                .args(args)
                .status()
                .expect("run git");
            assert!(status.success(), "git command failed: {args:?}");
        }

        fn commit(&self, message: &str) {
            self.git(&["add", "."]);
            self.git(&[
                "-c",
                "user.name=CodeM8 Test",
                "-c",
                "user.email=codem8@example.invalid",
                "commit",
                "-m",
                message,
            ]);
        }
    }

    impl Drop for TempGitRepo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    impl AsRef<Path> for TempGitRepo {
        fn as_ref(&self) -> &Path {
            &self.path
        }
    }

    fn run_in<P: AsRef<Path>>(
        project: P,
        args: &[&str],
    ) -> std::result::Result<String, CodeM8Error> {
        run_with_status(project, args).map(|(output, _status)| output)
    }

    fn run_with_status<P: AsRef<Path>>(
        project: P,
        args: &[&str],
    ) -> std::result::Result<(String, RunStatus), CodeM8Error> {
        let mut output = Vec::new();
        let status = run(args.iter().copied(), project.as_ref(), &mut output)?;
        Ok((String::from_utf8(output).expect("report is UTF-8"), status))
    }

    fn git_is_available() -> bool {
        Command::new("git")
            .arg("--version")
            .status()
            .is_ok_and(|status| status.success())
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
            [
                "Duplicate Code Report\n",
                "=====================\n",
                "\n",
                "Number of files analyzed: 2\n",
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
    fn duplicate_report_status_fails_when_duplicates_are_found() {
        let project = TempProject::new("duplicate-status");
        project.write("src/a.ts", "const value = one;\n");
        project.write("src/b.ts", "const value = one;\n");
        let (_output, status) =
            run_with_status(&project, &["--report-duplicate"]).expect("report succeeds");
        assert_eq!(status, RunStatus::IssuesFound);
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
        assert!(output.contains("Number of files analyzed: 1"));
        assert!(output.contains("Duplicate blocks found: 0"));
    }

    #[test]
    fn duplicate_report_status_succeeds_when_no_duplicates_are_found() {
        let project = TempProject::new("duplicate-clean-status");
        project.write("src/a.ts", "const first = one;\n");
        project.write("src/b.ts", "const second = two;\n");
        let (_output, status) =
            run_with_status(&project, &["--report-duplicate"]).expect("report succeeds");
        assert_eq!(status, RunStatus::Success);
    }

    #[test]
    fn verbose_explicit_files_report_lists_analyzed_files() {
        let project = TempProject::new("verbose-explicit-files");
        project.write("src/a.ts", "const value = one;\n");
        project.write("src/b.ts", "const value = one;\n");
        let quiet_output =
            run_in(&project, &["--report-duplicate", "-files=src/a.ts"]).expect("report succeeds");
        assert!(!quiet_output.contains("Files analyzed:"));
        let verbose_output = run_in(
            &project,
            &["--report-duplicate", "-verbose", "-files=src/a.ts"],
        )
        .expect("report succeeds");
        assert!(verbose_output.contains(
            "Number of files analyzed: 1\n\
             Files analyzed:\n\
             - src/a.ts\n\
             Analyzed extensions:"
        ));
    }

    #[test]
    fn verbose_recursive_duplicate_report_lists_analyzed_files() {
        let project = TempProject::new("verbose-recursive-duplicate");
        project.write("src/a.ts", "const first = one;\n");
        project.write("src/b.ts", "const second = two;\n");
        let output =
            run_in(&project, &["--report-duplicate", "-verbose"]).expect("report succeeds");
        assert!(output.contains(
            "Number of files analyzed: 2\n\
             Files analyzed:\n\
             - src/a.ts\n\
             - src/b.ts\n\
             Analyzed extensions:"
        ));
    }

    #[test]
    fn custom_extensions_change_analyzed_files() {
        let project = TempProject::new("custom-extensions");
        project.write("src/a.js", "const value = one;\n");
        project.write("src/b.js", "const value = one;\n");
        let default_output = run_in(&project, &["--report-duplicate"]).expect("report succeeds");
        assert!(default_output.contains("Number of files analyzed: 2"));
        assert!(default_output.contains("Duplicate blocks found: 1"));
        let js_output = run_in(&project, &["--report-duplicate", "-file-extension=js"])
            .expect("report succeeds");
        assert!(js_output.contains("Number of files analyzed: 2"));
        assert!(js_output.contains("Duplicate blocks found: 1"));
    }

    #[test]
    fn git_branch_mode_reports_duplicates_for_changed_files_against_repo() {
        if !git_is_available() {
            return;
        }
        let project = TempGitRepo::new("git-branch-scope");
        project.git(&["init"]);
        project.write("src/a.ts", "const original = 1;\n");
        project.write("src/b.ts", "const shared = 1;\n");
        project.commit("initial");
        project.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        project.git(&["branch", "-M", "feature"]);
        project.write("src/a.ts", "const shared = 1;\n");
        let output =
            run_in(&project, &["--report-duplicate", "-git-branch"]).expect("report succeeds");
        assert!(output.contains("Number of files analyzed: 1"));
        assert!(output.contains("Duplicate blocks found: 1"));
        assert!(output.contains("- src/a.ts:1-1"));
        assert!(output.contains("- src/b.ts:1-1"));
    }

    #[test]
    fn git_branch_mode_excludes_duplicates_without_changed_files() {
        if !git_is_available() {
            return;
        }
        let project = TempGitRepo::new("git-branch-duplicate-filter");
        project.git(&["init"]);
        project.write("src/a.ts", "const branch = 1;\n");
        project.write("src/b.ts", "const shared = 1;\n");
        project.write("src/c.ts", "const shared = 1;\n");
        project.commit("initial");
        project.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        project.git(&["branch", "-M", "feature"]);
        project.write("src/a.ts", "const branch = 2;\n");
        let output =
            run_in(&project, &["--report-duplicate", "-git-branch"]).expect("report succeeds");
        assert!(output.contains("Number of files analyzed: 1"));
        assert!(output.contains("Duplicate blocks found: 0"));
    }

    #[test]
    fn strict_git_branch_mode_reports_duplicates_only_on_changed_lines() {
        if !git_is_available() {
            return;
        }
        let project = TempGitRepo::new("strict-duplicate-lines");
        project.git(&["init"]);
        project.write("src/a.ts", "const shared = 1;\nconst branch = 1;\n");
        project.write("src/b.ts", "const shared = 1;\n");
        project.commit("initial");
        project.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        project.git(&["branch", "-M", "feature"]);
        project.write("src/a.ts", "const shared = 1;\nconst branch = 2;\n");
        let output = run_in(&project, &["--report-duplicate", "-git-branch-strict"])
            .expect("report succeeds");
        assert!(output.contains("Number of files analyzed: 1"));
        assert!(output.contains("Duplicate blocks found: 0"));
        project.write("src/a.ts", "const changed = 1;\nconst branch = 2;\n");
        project.commit("branch change");
        project.write("src/b.ts", "const changed = 1;\n");
        let output = run_in(&project, &["--report-duplicate", "-git-branch-strict"])
            .expect("report succeeds");
        assert!(output.contains("Duplicate blocks found: 1"));
        assert!(output.contains("- src/a.ts:1-1"));
        assert!(output.contains("- src/b.ts:1-1"));
    }

    #[test]
    fn verbose_strict_git_branch_report_lists_changed_line_ranges() {
        if !git_is_available() {
            return;
        }
        let project = TempGitRepo::new("strict-verbose-ranges");
        project.git(&["init"]);
        project.write(
            "src/a.ts",
            "const one = 1;\nconst two = 2;\nconst three = 3;\n",
        );
        project.commit("initial");
        project.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        project.git(&["branch", "-M", "feature"]);
        project.write(
            "src/a.ts",
            "const one = 1;\nconst two = 20;\nconst three = 30;\n",
        );
        let output = run_in(
            &project,
            &["--report-duplicate", "-git-branch-strict", "-verbose"],
        )
        .expect("report succeeds");
        assert!(output.contains("Files analyzed:\n- src/a.ts (2-3)\n"));
    }

    #[test]
    fn complexity_report_lists_functions_over_limits() {
        let project = TempProject::new("complexity");
        project.write(
            "src/lib.rs",
            "fn risky(value: i32) -> i32 {\n\
             if value > 10 {\n\
             return 10;\n\
             }\n\
             if value > 5 {\n\
             return 5;\n\
             }\n\
             0\n\
             }\n",
        );
        let output = run_in(
            &project,
            [
                "--report-complexity",
                "-file-extension=rs",
                "-max-cognitive-complexity=1",
                "-max-cyclomatic-complexity=1",
            ]
            .as_slice(),
        )
        .expect("report succeeds");
        assert!(output.contains("Complexity Report"));
        assert!(output.contains("Number of files analyzed: 1"));
        assert!(output.contains("Functions exceeding limits: 1"));
        assert!(output.contains("Function: risky"));
        assert!(output.contains("Location: src/lib.rs:1-9"));
        assert!(output.contains("Cognitive complexity:"));
        assert!(output.contains("Cyclomatic complexity:"));
    }

    #[test]
    fn complexity_report_status_fails_when_complex_functions_are_found() {
        let project = TempProject::new("complexity-status");
        project.write(
            "src/lib.rs",
            "fn risky(value: i32) -> i32 {\n\
             if value > 10 {\n\
             return 10;\n\
             }\n\
             if value > 5 {\n\
             return 5;\n\
             }\n\
             0\n\
             }\n",
        );
        let (_output, status) = run_with_status(
            &project,
            &[
                "--report-complexity",
                "-file-extension=rs",
                "-max-cognitive-complexity=1",
                "-max-cyclomatic-complexity=1",
            ],
        )
        .expect("report succeeds");
        assert_eq!(status, RunStatus::IssuesFound);
    }

    #[test]
    fn complexity_report_skips_unsupported_extensions() {
        let project = TempProject::new("complexity-unsupported");
        project.write("src/lib.rb", "def risky\nend\n");
        let output = run_in(&project, &["--report-complexity"]).expect("report succeeds");
        assert!(output.contains("Number of files analyzed: 0"));
        assert!(output.contains("Functions exceeding limits: 0"));
    }

    #[test]
    fn verbose_recursive_complexity_report_lists_analyzed_files() {
        let project = TempProject::new("verbose-recursive-complexity");
        project.write("src/main.rs", "fn main() {\n}\n");
        project.write("src/lib.rs", "fn lib() {\n}\n");
        let output =
            run_in(&project, &["--report-complexity", "-verbose"]).expect("report succeeds");
        assert!(output.contains(
            "Number of files analyzed: 2\n\
             Files analyzed:\n\
             - src/lib.rs\n\
             - src/main.rs\n\
             Analyzed extensions:"
        ));
    }

    #[test]
    fn help_status_succeeds() {
        let project = TempProject::new("help-status");
        let (_output, status) = run_with_status(&project, &["help"]).expect("help succeeds");
        assert_eq!(status, RunStatus::Success);
    }

    #[test]
    fn git_branch_mode_limits_complexity_search_to_changed_files() {
        if !git_is_available() {
            return;
        }
        let project = TempGitRepo::new("complexity-git-branch-scope");
        project.git(&["init"]);
        project.write(
            "src/unchanged.rs",
            "fn risky(value: i32) -> i32 {\n\
             if value > 10 {\n\
             return 10;\n\
             }\n\
             if value > 5 {\n\
             return 5;\n\
             }\n\
             0\n\
             }\n",
        );
        project.write("src/changed.rs", "fn simple() -> i32 {\n1\n}\n");
        project.commit("initial");
        project.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        project.git(&["branch", "-M", "feature"]);
        project.write("src/changed.rs", "fn simple() -> i32 {\n2\n}\n");
        let output = run_in(
            &project,
            [
                "--report-complexity",
                "-git-branch",
                "-file-extension=rs",
                "-max-cognitive-complexity=1",
                "-max-cyclomatic-complexity=1",
            ]
            .as_slice(),
        )
        .expect("report succeeds");
        assert!(output.contains("Number of files analyzed: 1"));
        assert!(output.contains("Functions exceeding limits: 0"));
    }

    #[test]
    fn strict_git_branch_mode_reports_complexity_only_for_changed_functions() {
        if !git_is_available() {
            return;
        }
        let project = TempGitRepo::new("strict-complexity-lines");
        project.git(&["init"]);
        project.write(
            "src/lib.rs",
            "fn risky(value: i32) -> i32 {\n\
             if value > 10 {\n\
             return 10;\n\
             }\n\
             if value > 5 {\n\
             return 5;\n\
             }\n\
             0\n\
             }\n\
             const VALUE: i32 = 1;\n",
        );
        project.commit("initial");
        project.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        project.git(&["branch", "-M", "feature"]);
        project.write(
            "src/lib.rs",
            "fn risky(value: i32) -> i32 {\n\
             if value > 10 {\n\
             return 10;\n\
             }\n\
             if value > 5 {\n\
             return 5;\n\
             }\n\
             0\n\
             }\n\
             const VALUE: i32 = 2;\n",
        );
        let output = run_in(
            &project,
            &[
                "--report-complexity",
                "-git-branch-strict",
                "-file-extension=rs",
                "-max-cognitive-complexity=1",
                "-max-cyclomatic-complexity=1",
            ],
        )
        .expect("report succeeds");
        assert!(output.contains("Number of files analyzed: 1"));
        assert!(output.contains("Functions exceeding limits: 0"));
        project.write(
            "src/lib.rs",
            "fn risky(value: i32) -> i32 {\n\
             if value > 10 {\n\
             return 11;\n\
             }\n\
             if value > 5 {\n\
             return 5;\n\
             }\n\
             0\n\
             }\n\
             const VALUE: i32 = 2;\n",
        );
        let output = run_in(
            &project,
            &[
                "--report-complexity",
                "-git-branch-strict",
                "-file-extension=rs",
                "-max-cognitive-complexity=1",
                "-max-cyclomatic-complexity=1",
            ],
        )
        .expect("report succeeds");
        assert!(output.contains("Functions exceeding limits: 1"));
        assert!(output.contains("Function: risky"));
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

    #[test]
    fn version_option_prints_current_version() {
        let project = TempProject::new("version");
        let output = run_in(&project, &["--version"]).expect("version succeeds");
        assert_eq!(output, "0.7.7\n");
    }
}
