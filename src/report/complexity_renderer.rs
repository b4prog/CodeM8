use std::fmt::Write as _;
use std::time::Duration;

use crate::model::{AnalyzedFile, FunctionComplexity, LineRange};
use crate::paths::format_path;

#[derive(Debug, Clone, PartialEq)]
pub struct ComplexityReport {
    pub analyzed_files: usize,
    pub analyzed_extensions: Vec<String>,
    pub analyzed_file_paths: Option<Vec<AnalyzedFile>>,
    pub max_cognitive_complexity: u32,
    pub max_cyclomatic_complexity: u32,
    pub timings: Option<ComplexityReportTimings>,
    pub functions: Vec<FunctionComplexity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComplexityReportTimings {
    pub discovery: Duration,
    pub complexity_analysis: Duration,
}

#[must_use]
pub fn render_complexity_report(report: &ComplexityReport, verbose: bool) -> String {
    let mut output = String::new();
    output.push_str("Complexity Report\n");
    output.push_str("=================\n\n");
    let _ = writeln!(
        output,
        "Number of files analyzed: {}",
        report.analyzed_files
    );
    if verbose {
        render_analyzed_files(&mut output, report.analyzed_file_paths.as_deref());
        render_verbose_summary(&mut output, report);
    }
    let _ = writeln!(
        output,
        "Functions exceeding limits: {}",
        report.functions.len()
    );
    if verbose {
        render_timings(&mut output, report.timings);
    }
    for (index, function) in report.functions.iter().enumerate() {
        output.push('\n');
        let _ = writeln!(output, "#{}", index + 1);
        let _ = writeln!(output, "Function: {}", function.function_name);
        let _ = writeln!(
            output,
            "Location: {}:{}-{}",
            format_path(&function.file_path),
            function.start_line,
            function.end_line
        );
        let _ = writeln!(
            output,
            "Cognitive complexity: {}",
            format_metric(function.cognitive_complexity)
        );
        let _ = writeln!(
            output,
            "Cyclomatic complexity: {}",
            format_metric(function.cyclomatic_complexity)
        );
    }
    output
}

fn render_verbose_summary(output: &mut String, report: &ComplexityReport) {
    let _ = writeln!(
        output,
        "Analyzed extensions: {}",
        sorted_extensions(&report.analyzed_extensions).join(", ")
    );
    let _ = writeln!(
        output,
        "Max cognitive complexity: {}",
        report.max_cognitive_complexity
    );
    let _ = writeln!(
        output,
        "Max cyclomatic complexity: {}",
        report.max_cyclomatic_complexity
    );
}

fn render_analyzed_files(output: &mut String, analyzed_file_paths: Option<&[AnalyzedFile]>) {
    if let Some(analyzed_file_paths) = analyzed_file_paths {
        output.push_str("Files analyzed:\n");
        for file in analyzed_file_paths {
            let _ = writeln!(output, "- {}", format_analyzed_file(file));
        }
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

fn render_timings(output: &mut String, timings: Option<ComplexityReportTimings>) {
    if let Some(timings) = timings {
        output.push_str("Timings:\n");
        let _ = writeln!(
            output,
            "- Discovery: {}",
            format_duration(timings.discovery)
        );
        let _ = writeln!(
            output,
            "- Complexity analysis: {}",
            format_duration(timings.complexity_analysis)
        );
    }
}

fn format_metric(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
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

    use super::*;

    #[test]
    fn renders_empty_report() {
        let report = ComplexityReport {
            analyzed_files: 0,
            analyzed_extensions: vec!["rs".to_string()],
            analyzed_file_paths: None,
            max_cognitive_complexity: 15,
            max_cyclomatic_complexity: 10,
            timings: None,
            functions: Vec::new(),
        };
        assert_eq!(
            render_complexity_report(&report, false),
            "Complexity Report\n\
             =================\n\
             \n\
             Number of files analyzed: 0\n\
             Functions exceeding limits: 0\n"
        );
    }

    #[test]
    fn renders_function_metrics() {
        let report = ComplexityReport {
            analyzed_files: 1,
            analyzed_extensions: vec!["rs".to_string()],
            analyzed_file_paths: None,
            max_cognitive_complexity: 15,
            max_cyclomatic_complexity: 10,
            timings: None,
            functions: vec![FunctionComplexity {
                file_path: PathBuf::from("src/lib.rs"),
                function_name: "run".to_string(),
                start_line: 10,
                end_line: 20,
                cognitive_complexity: 16.0,
                cyclomatic_complexity: 8.0,
            }],
        };
        let output = render_complexity_report(&report, false);
        assert!(output.contains("#1\n"));
        assert!(output.contains("Function: run\n"));
        assert!(output.contains("Location: src/lib.rs:10-20\n"));
        assert!(output.contains("Cognitive complexity: 16\n"));
        assert!(output.contains("Cyclomatic complexity: 8\n"));
    }

    #[test]
    fn renders_verbose_files_and_timings() {
        let report = ComplexityReport {
            analyzed_files: 1,
            analyzed_extensions: vec!["rs".to_string()],
            analyzed_file_paths: Some(vec![AnalyzedFile {
                path: PathBuf::from("src/lib.rs"),
                changed_lines: None,
            }]),
            max_cognitive_complexity: 15,
            max_cyclomatic_complexity: 10,
            timings: Some(ComplexityReportTimings {
                discovery: Duration::from_micros(1_234),
                complexity_analysis: Duration::from_micros(12_345),
            }),
            functions: Vec::new(),
        };
        let output = render_complexity_report(&report, true);
        assert!(output.contains("Files analyzed:\n- src/lib.rs\n"));
        assert!(output.contains("Analyzed extensions: rs\n"));
        assert!(output.contains("Max cognitive complexity: 15\n"));
        assert!(output.contains("Max cyclomatic complexity: 10\n"));
        assert!(output.contains("- Discovery: 1.234 ms\n"));
        assert!(output.contains("- Complexity analysis: 12.345 ms\n"));
    }

    #[test]
    fn renders_verbose_changed_line_ranges() {
        let report = ComplexityReport {
            analyzed_files: 1,
            analyzed_extensions: vec!["rs".to_string()],
            analyzed_file_paths: Some(vec![AnalyzedFile {
                path: PathBuf::from("src/lib.rs"),
                changed_lines: Some(vec![
                    LineRange { start: 3, end: 17 },
                    LineRange { start: 21, end: 21 },
                ]),
            }]),
            max_cognitive_complexity: 15,
            max_cyclomatic_complexity: 10,
            timings: None,
            functions: Vec::new(),
        };
        let output = render_complexity_report(&report, true);
        assert!(output.contains("Files analyzed:\n- src/lib.rs (3-17,21)\n"));
    }
}
