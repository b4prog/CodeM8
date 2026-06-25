use crate::model::DuplicateBlock;
use crate::paths::format_path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateReport {
    pub analyzed_files: usize,
    pub analyzed_extensions: Vec<String>,
    pub duplicate_blocks: Vec<DuplicateBlock>,
}

pub fn render_duplicate_report(report: &DuplicateReport) -> String {
    let mut output = String::new();
    output.push_str("Duplicate Code Report\n");
    output.push_str("=====================\n\n");
    output.push_str(&format!("Analyzed files: {}\n", report.analyzed_files));
    output.push_str(&format!(
        "Analyzed extensions: {}\n",
        report.analyzed_extensions.join(", ")
    ));
    output.push_str(&format!(
        "Duplicate blocks found: {}\n",
        report.duplicate_blocks.len()
    ));
    for (index, block) in report.duplicate_blocks.iter().enumerate() {
        output.push('\n');
        output.push_str(&format!("#{} Weight: {}\n", index + 1, block.weight));
        output.push_str(&format!("Lines: {}\n", block.line_count()));
        output.push_str(&format!("Characters: {}\n", block.character_count()));
        output.push_str(&format!("Occurrences: {}\n\n", block.occurrences.len()));
        output.push_str("Locations:\n");
        for occurrence in &block.occurrences {
            output.push_str(&format!(
                "- {}:{}-{}\n",
                format_path(&occurrence.file_path),
                occurrence.start_line,
                occurrence.end_line
            ));
        }
        output.push_str("\nCode:\n");
        for line in &block.normalized_lines {
            output.push_str("  ");
            output.push_str(line);
            output.push('\n');
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
            render_duplicate_report(&report),
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
        let output = render_duplicate_report(&report);
        assert!(output.contains("#1 Weight: 13"));
        assert!(output.contains("Lines: 1"));
        assert!(output.contains("- src/a.ts:1-1"));
        assert!(output.contains("  return value;"));
    }
}
