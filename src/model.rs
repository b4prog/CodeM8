use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStatus {
    Comparison,
    BlockOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub path: PathBuf,
    pub display_path: PathBuf,
    pub extension: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineEntry {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub normalized_text: String,
    pub hash: u128,
    pub status: LineStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessedFile {
    pub source: SourceFile,
    pub lines: Vec<LineEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateOccurrence {
    pub file_path: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateBlock {
    pub normalized_lines: Vec<String>,
    pub occurrences: Vec<DuplicateOccurrence>,
    pub weight: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionComplexity {
    pub file_path: PathBuf,
    pub function_name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub cognitive_complexity: f64,
    pub cyclomatic_complexity: f64,
}

impl DuplicateBlock {
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.normalized_lines.len()
    }

    #[must_use]
    pub fn character_count(&self) -> u64 {
        self.normalized_lines
            .iter()
            .map(|line| line.chars().count() as u64)
            .sum()
    }
}
