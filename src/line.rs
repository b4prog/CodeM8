use std::fs::File;
use std::io::{BufRead, BufReader};

use rayon::prelude::*;

use crate::error::{CodeM8Error, Result};
use crate::language::{classify_line, hash_normalized_line};
use crate::model::{LineEntry, ProcessedFile, SourceFile};

/// Processes a set of source files into normalized line entries.
///
/// # Errors
///
/// Returns an error when any input file cannot be opened or read as UTF-8 text.
pub fn process_source_files(source_files: &[SourceFile]) -> Result<Vec<ProcessedFile>> {
    source_files.par_iter().map(process_source_file).collect()
}

/// Processes one source file into its normalized, classified lines.
///
/// # Errors
///
/// Returns an error when the file cannot be opened or read as UTF-8 text.
pub fn process_source_file(source_file: &SourceFile) -> Result<ProcessedFile> {
    let file = File::open(&source_file.path)
        .map_err(|error| CodeM8Error::io(&source_file.display_path, "open file", &error))?;
    let reader = BufReader::new(file);
    let mut lines = Vec::new();
    for (index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| {
            CodeM8Error::new(format!(
                "could not read {} as UTF-8 text: {error}",
                crate::paths::format_path(&source_file.display_path)
            ))
        })?;
        let Some(normalized_text) = normalize_line(&line) else {
            continue;
        };
        let hash = hash_normalized_line(&normalized_text);
        let status = classify_line(&source_file.extension, &normalized_text, hash);
        lines.push(LineEntry {
            file_path: source_file.display_path.clone(),
            line_number: index + 1,
            normalized_text,
            hash,
            status,
        });
    }
    Ok(ProcessedFile {
        source: source_file.clone(),
        lines,
    })
}

#[must_use]
pub fn normalize_line(line: &str) -> Option<String> {
    let normalized = line.trim();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::model::LineStatus;

    use super::*;

    #[test]
    fn trims_unicode_whitespace_and_skips_empty_lines() {
        assert_eq!(
            normalize_line("\t value \u{2003}"),
            Some("value".to_string())
        );
        assert_eq!(normalize_line(" \t "), None);
    }

    #[test]
    fn processes_non_empty_lines_with_original_line_numbers() {
        let path = std::env::temp_dir().join(format!("codem8-line-test-{}.ts", std::process::id()));
        fs::write(&path, "  const value = 1;  \n\n   }\n").expect("write source file");
        let source = SourceFile {
            path: path.clone(),
            display_path: "sample.ts".into(),
            extension: "ts".to_string(),
        };
        let processed = process_source_file(&source).expect("process source file");
        assert_eq!(processed.lines.len(), 2);
        assert_eq!(processed.lines[0].line_number, 1);
        assert_eq!(processed.lines[0].normalized_text, "const value = 1;");
        assert_eq!(processed.lines[0].status, LineStatus::Comparison);
        assert_eq!(processed.lines[1].line_number, 3);
        assert_eq!(processed.lines[1].normalized_text, "}");
        assert_eq!(processed.lines[1].status, LineStatus::BlockOnly);
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn processes_files_in_input_order() {
        let id = std::process::id();
        let first_path = std::env::temp_dir().join(format!("codem8-line-order-first-{id}.ts"));
        let second_path = std::env::temp_dir().join(format!("codem8-line-order-second-{id}.ts"));
        fs::write(&first_path, "const first = 1;\n").expect("write first source file");
        fs::write(&second_path, "const second = 2;\n").expect("write second source file");
        let sources = vec![
            SourceFile {
                path: first_path.clone(),
                display_path: "first.ts".into(),
                extension: "ts".to_string(),
            },
            SourceFile {
                path: second_path.clone(),
                display_path: "second.ts".into(),
                extension: "ts".to_string(),
            },
        ];
        let processed = process_source_files(&sources).expect("process source files");
        assert_eq!(processed[0].source.display_path, PathBuf::from("first.ts"));
        assert_eq!(processed[1].source.display_path, PathBuf::from("second.ts"));
        fs::remove_file(first_path).expect("cleanup first");
        fs::remove_file(second_path).expect("cleanup second");
    }

    #[test]
    fn returns_clear_error_for_invalid_utf8() {
        let path = std::env::temp_dir().join(format!(
            "codem8-line-invalid-utf8-{}.ts",
            std::process::id()
        ));
        fs::write(&path, [0xff, b'\n']).expect("write invalid source file");
        let source = SourceFile {
            path: path.clone(),
            display_path: "invalid.ts".into(),
            extension: "ts".to_string(),
        };
        let error = process_source_file(&source).expect_err("invalid UTF-8 fails");
        assert!(error
            .to_string()
            .contains("could not read invalid.ts as UTF-8 text"));
        fs::remove_file(path).expect("cleanup");
    }
}
