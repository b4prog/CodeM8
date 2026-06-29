use std::fmt::Write as _;
use std::time::Duration;

use crate::model::{AnalyzedFile, DuplicateBlock, LineRange};
use crate::paths::format_path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateReport {
    pub analyzed_files: usize,
    pub analyzed_extensions: Vec<String>,
    pub analyzed_file_paths: Option<Vec<AnalyzedFile>>,
    pub timings: Option<DuplicateReportTimings>,
    pub duplicate_blocks: Vec<DuplicateBlock>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuplicateReportTimings {
    pub discovery: Duration,
    pub file_processing: Duration,
    pub duplicate_detection: Duration,
}

#[must_use]
pub fn render_duplicate_report(report: &DuplicateReport, verbose: bool) -> String {
    let mut output = String::new();
    render_report_summary(&mut output, report, verbose);
    render_duplicate_blocks(&mut output, &report.duplicate_blocks, verbose);
    output
}

fn render_report_summary(output: &mut String, report: &DuplicateReport, verbose: bool) {
    output.push_str("Duplicate Code Report\n");
    output.push_str("=====================\n\n");
    let _ = writeln!(
        output,
        "Number of files analyzed: {}",
        report.analyzed_files
    );
    let analyzed_file_paths = if verbose {
        report.analyzed_file_paths.as_ref()
    } else {
        None
    };
    if let Some(analyzed_file_paths) = analyzed_file_paths {
        output.push_str("Files analyzed:\n");
        render_analyzed_files(output, analyzed_file_paths);
    }
    if verbose {
        render_verbose_summary(output, report);
    }
    let _ = writeln!(
        output,
        "Duplicate blocks found: {}",
        report.duplicate_blocks.len()
    );
    if verbose {
        render_timings(output, report.timings);
    }
}

fn render_verbose_summary(output: &mut String, report: &DuplicateReport) {
    let _ = writeln!(
        output,
        "Analyzed extensions: {}",
        sorted_extensions(&report.analyzed_extensions).join(", ")
    );
}

fn render_analyzed_files(output: &mut String, analyzed_file_paths: &[AnalyzedFile]) {
    for file in analyzed_file_paths {
        let _ = writeln!(output, "- {}", format_analyzed_file(file));
    }
}

fn format_analyzed_file(file: &AnalyzedFile) -> String {
    match file.changed_lines.as_deref() {
        Some(lines) if !lines.is_empty() => {
            format!(
                "{} ({})",
                format_path(&file.path),
                format_line_ranges(lines)
            )
        }
        Some(_) | None => format_path(&file.path),
    }
}

fn format_line_ranges(lines: &[LineRange]) -> String {
    lines
        .iter()
        .map(format_line_range)
        .collect::<Vec<_>>()
        .join(",")
}

fn format_line_range(range: &LineRange) -> String {
    if range.start == range.end {
        range.start.to_string()
    } else {
        format!("{}-{}", range.start, range.end)
    }
}

fn render_timings(output: &mut String, timings: Option<DuplicateReportTimings>) {
    if let Some(timings) = timings {
        output.push_str("Timings:\n");
        let _ = writeln!(
            output,
            "- Discovery: {}",
            format_duration(timings.discovery)
        );
        let _ = writeln!(
            output,
            "- File processing: {}",
            format_duration(timings.file_processing)
        );
        let _ = writeln!(
            output,
            "- Duplicate detection: {}",
            format_duration(timings.duplicate_detection)
        );
    }
}

fn render_duplicate_blocks(output: &mut String, blocks: &[DuplicateBlock], verbose: bool) {
    for (index, block) in blocks.iter().enumerate() {
        output.push('\n');
        let _ = writeln!(output, "#{}", index + 1);
        if verbose {
            render_verbose_block(output, block);
        }
        render_block_locations(output, block);
    }
}

fn render_verbose_block(output: &mut String, block: &DuplicateBlock) {
    let _ = writeln!(output, "Weight: {}", block.weight);
    let _ = writeln!(output, "Lines: {}", block.line_count());
    let _ = writeln!(output, "Occurrences: {}", block.occurrences.len());
    output.push('\n');
    output.push_str("Code:\n");
    for line in &block.normalized_lines {
        output.push_str("  ");
        output.push_str(line);
        output.push('\n');
    }
    output.push_str("\nLocations:\n");
}

fn render_block_locations(output: &mut String, block: &DuplicateBlock) {
    for occurrence in &block.occurrences {
        let _ = writeln!(
            output,
            "- {}:{}-{}",
            format_path(&occurrence.file_path),
            occurrence.start_line,
            occurrence.end_line
        );
    }
}

fn format_duration(duration: Duration) -> String {
    let microseconds = duration.as_micros();
    let milliseconds = microseconds / 1_000;
    let fractional_microseconds = microseconds % 1_000;
    format!("{milliseconds}.{fractional_microseconds:03} ms")
}

fn sorted_extensions(extensions: &[String]) -> Vec<String> {
    let mut extensions = extensions.to_vec();
    extensions.sort();
    extensions
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use crate::model::{AnalyzedFile, DuplicateBlock, DuplicateOccurrence};

    use super::*;

    #[test]
    fn renders_empty_report() {
        let report = DuplicateReport {
            analyzed_files: 0,
            analyzed_extensions: vec!["ts".to_string()],
            analyzed_file_paths: None,
            timings: None,
            duplicate_blocks: Vec::new(),
        };
        assert_eq!(
            render_duplicate_report(&report, false),
            "Duplicate Code Report\n\
             =====================\n\
             \n\
             Number of files analyzed: 0\n\
             Duplicate blocks found: 0\n"
        );
    }

    #[test]
    fn renders_duplicate_block_details() {
        let report = DuplicateReport {
            analyzed_files: 2,
            analyzed_extensions: vec!["ts".to_string(), "js".to_string()],
            analyzed_file_paths: None,
            timings: None,
            duplicate_blocks: vec![DuplicateBlock {
                normalized_lines: vec!["return value;".to_string()],
                occurrences: vec![
                    DuplicateOccurrence {
                        file_path: PathBuf::from("src/a.ts"),
                        start_line: 1,
                        end_line: 1,
                    },
                    DuplicateOccurrence {
                        file_path: PathBuf::from("src/b.js"),
                        start_line: 5,
                        end_line: 5,
                    },
                ],
                weight: 13,
            }],
        };
        let output = render_duplicate_report(&report, false);
        assert!(output.contains("#1\n"));
        assert!(!output.contains("Weight: 13"));
        assert!(!output.contains("Lines: 1"));
        assert!(!output.contains("Occurrences: 2"));
        assert!(!output.contains("Characters:"));
        assert!(!output.contains("Code:"));
        assert!(!output.contains("Locations:"));
        assert!(output.contains("- src/a.ts:1-1"));
        assert!(!output.contains("  return value;"));
        assert!(output.contains("#1\n- src/a.ts:1-1\n- src/b.js:5-5\n"));
    }

    #[test]
    fn renders_analyzed_extensions_alphabetically() {
        let report = DuplicateReport {
            analyzed_files: 0,
            analyzed_extensions: vec!["ts".to_string(), "js".to_string(), "rs".to_string()],
            analyzed_file_paths: None,
            timings: None,
            duplicate_blocks: Vec::new(),
        };
        let output = render_duplicate_report(&report, false);
        assert!(!output.contains("Analyzed extensions:"));
        let verbose_output = render_duplicate_report(&report, true);
        assert!(verbose_output.contains("Analyzed extensions: js, rs, ts\n"));
    }

    #[test]
    fn renders_duplicate_block_metrics_in_verbose_mode() {
        let report = DuplicateReport {
            analyzed_files: 2,
            analyzed_extensions: vec!["ts".to_string()],
            analyzed_file_paths: None,
            timings: None,
            duplicate_blocks: vec![DuplicateBlock {
                normalized_lines: vec!["return value;".to_string()],
                occurrences: vec![
                    DuplicateOccurrence {
                        file_path: PathBuf::from("src/a.ts"),
                        start_line: 1,
                        end_line: 1,
                    },
                    DuplicateOccurrence {
                        file_path: PathBuf::from("src/b.ts"),
                        start_line: 2,
                        end_line: 2,
                    },
                ],
                weight: 13,
            }],
        };
        let output = render_duplicate_report(&report, true);
        assert!(output.contains("Weight: 13"));
        assert!(output.contains("Lines: 1"));
        assert!(output.contains("Occurrences: 2"));
        assert!(!output.contains("Characters:"));
        assert!(output.contains("Code:"));
        assert!(output.contains("Locations:"));
        assert!(output.contains("  return value;"));
    }

    #[test]
    fn renders_analyzed_file_list_only_in_verbose_mode() {
        let report = DuplicateReport {
            analyzed_files: 2,
            analyzed_extensions: vec!["ts".to_string()],
            analyzed_file_paths: Some(vec![
                AnalyzedFile {
                    path: PathBuf::from("src/a.ts"),
                    changed_lines: None,
                },
                AnalyzedFile {
                    path: PathBuf::from("src/nested/b.ts"),
                    changed_lines: None,
                },
            ]),
            timings: None,
            duplicate_blocks: Vec::new(),
        };
        let quiet_output = render_duplicate_report(&report, false);
        assert!(!quiet_output.contains("Files analyzed:"));
        let verbose_output = render_duplicate_report(&report, true);
        assert!(verbose_output.contains(
            "Number of files analyzed: 2\n\
             Files analyzed:\n\
             - src/a.ts\n\
             - src/nested/b.ts\n\
             Analyzed extensions: ts"
        ));
    }

    #[test]
    fn renders_timings_only_in_verbose_mode() {
        let report = DuplicateReport {
            analyzed_files: 1,
            analyzed_extensions: vec!["ts".to_string()],
            analyzed_file_paths: None,
            timings: Some(DuplicateReportTimings {
                discovery: Duration::from_micros(1_234),
                file_processing: Duration::from_micros(12_345),
                duplicate_detection: Duration::from_micros(123_456),
            }),
            duplicate_blocks: Vec::new(),
        };
        let quiet_output = render_duplicate_report(&report, false);
        assert!(!quiet_output.contains("Timings:"));
        let verbose_output = render_duplicate_report(&report, true);
        assert!(verbose_output.contains(
            "Timings:\n\
             - Discovery: 1.234 ms\n\
             - File processing: 12.345 ms\n\
             - Duplicate detection: 123.456 ms\n"
        ));
    }
}
