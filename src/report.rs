use std::fmt::Write as _;

use crate::model::DuplicateBlock;
use crate::paths::format_path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateReport {
    pub analyzed_files: usize,
    pub analyzed_extensions: Vec<String>,
    pub duplicate_blocks: Vec<DuplicateBlock>,
}

#[must_use]
pub fn render_duplicate_report(report: &DuplicateReport, verbose: bool) -> String {
    let mut output = String::new();
    output.push_str("Duplicate Code Report\n");
    output.push_str("=====================\n\n");
    let _ = writeln!(output, "Analyzed files: {}", report.analyzed_files);
    let _ = writeln!(
        output,
        "Analyzed extensions: {}",
        report.analyzed_extensions.join(", ")
    );
    let _ = writeln!(
        output,
        "Duplicate blocks found: {}",
        report.duplicate_blocks.len()
    );
    for (index, block) in report.duplicate_blocks.iter().enumerate() {
        output.push('\n');
        let _ = writeln!(output, "#{}", index + 1);
        if verbose {
            let _ = writeln!(output, "Weight: {}", block.weight);
            let _ = writeln!(output, "Lines: {}", block.line_count());
            let _ = writeln!(output, "Occurrences: {}", block.occurrences.len());
            output.push('\n');
        }
        output.push_str("Code:\n");
        for line in &block.normalized_lines {
            output.push_str("  ");
            output.push_str(line);
            output.push('\n');
        }
        output.push_str("\nLocations:\n");
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
    output
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::model::{DuplicateBlock, DuplicateOccurrence};

    use super::*;

    #[test]
    fn renders_empty_report() {
        let report = DuplicateReport {
            analyzed_files: 0,
            analyzed_extensions: vec!["ts".to_string()],
            duplicate_blocks: Vec::new(),
        };
        assert_eq!(
            render_duplicate_report(&report, false),
            "Duplicate Code Report\n\
             =====================\n\
             \n\
             Analyzed files: 0\n\
             Analyzed extensions: ts\n\
             Duplicate blocks found: 0\n"
        );
    }

    #[test]
    fn renders_duplicate_block_details() {
        let report = DuplicateReport {
            analyzed_files: 2,
            analyzed_extensions: vec!["ts".to_string(), "js".to_string()],
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
        assert!(output.contains("- src/a.ts:1-1"));
        assert!(output.contains("  return value;"));
        assert!(
            output.find("Code:").expect("code section exists")
                < output.find("Locations:").expect("locations section exists")
        );
    }

    #[test]
    fn renders_duplicate_block_metrics_in_verbose_mode() {
        let report = DuplicateReport {
            analyzed_files: 2,
            analyzed_extensions: vec!["ts".to_string()],
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
    }
}
