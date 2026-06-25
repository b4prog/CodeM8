use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use crate::model::{DuplicateBlock, DuplicateOccurrence, LineEntry, LineStatus, ProcessedFile};
use crate::paths::format_path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct LineRef {
    file_index: usize,
    line_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OccurrenceKey {
    file_path: PathBuf,
    file_path_key: String,
    start_line: usize,
    end_line: usize,
}

impl Ord for OccurrenceKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.file_path_key
            .cmp(&other.file_path_key)
            .then_with(|| self.start_line.cmp(&other.start_line))
            .then_with(|| self.end_line.cmp(&other.end_line))
    }
}

impl PartialOrd for OccurrenceKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn detect_duplicate_blocks(files: &[ProcessedFile]) -> Vec<DuplicateBlock> {
    let mut line_index: HashMap<u128, Vec<LineRef>> = HashMap::new();
    for (file_index, file) in files.iter().enumerate() {
        for (line_index_in_file, line) in file.lines.iter().enumerate() {
            line_index.entry(line.hash).or_default().push(LineRef {
                file_index,
                line_index: line_index_in_file,
            });
        }
    }
    let mut blocks_by_lines: HashMap<Vec<String>, BTreeSet<OccurrenceKey>> = HashMap::new();
    for refs in line_index.values() {
        if refs.len() < 2 {
            continue;
        }
        let mut comparison_refs_by_text: HashMap<String, Vec<LineRef>> = HashMap::new();
        for line_ref in refs {
            let line = line_at(files, *line_ref);
            if line.status != LineStatus::Comparison {
                continue;
            }
            comparison_refs_by_text
                .entry(line.normalized_text.clone())
                .or_default()
                .push(*line_ref);
        }
        for comparison_refs in comparison_refs_by_text.values() {
            if comparison_refs.len() < 2 {
                continue;
            }
            for left_index in 0..comparison_refs.len() {
                for right_index in (left_index + 1)..comparison_refs.len() {
                    let left = comparison_refs[left_index];
                    let right = comparison_refs[right_index];
                    let Some(candidate) = expand_pair(files, left, right) else {
                        continue;
                    };
                    let occurrences = blocks_by_lines
                        .entry(candidate.normalized_lines)
                        .or_default();
                    occurrences.insert(candidate.left_occurrence);
                    occurrences.insert(candidate.right_occurrence);
                }
            }
        }
    }
    let mut duplicate_blocks = blocks_by_lines
        .into_iter()
        .filter_map(|(normalized_lines, occurrences)| {
            if normalized_lines.is_empty() || occurrences.len() < 2 {
                return None;
            }
            let occurrences = occurrences
                .into_iter()
                .map(|occurrence| DuplicateOccurrence {
                    file_path: occurrence.file_path,
                    start_line: occurrence.start_line,
                    end_line: occurrence.end_line,
                })
                .collect::<Vec<_>>();
            let character_count = normalized_lines
                .iter()
                .map(|line| line.chars().count() as u64)
                .sum::<u64>();
            let weight =
                (occurrences.len() as u64 - 1) * normalized_lines.len() as u64 * character_count;
            Some(DuplicateBlock {
                normalized_lines,
                occurrences,
                weight,
            })
        })
        .collect::<Vec<_>>();
    duplicate_blocks.sort_by(compare_duplicate_blocks);
    duplicate_blocks
}

#[derive(Debug)]
struct CandidateBlock {
    normalized_lines: Vec<String>,
    left_occurrence: OccurrenceKey,
    right_occurrence: OccurrenceKey,
}

fn expand_pair(files: &[ProcessedFile], left: LineRef, right: LineRef) -> Option<CandidateBlock> {
    if left == right {
        return None;
    }
    let mut left_start = left.line_index;
    let mut right_start = right.line_index;
    while left_start > 0
        && right_start > 0
        && line_text(files, left.file_index, left_start - 1)
            == line_text(files, right.file_index, right_start - 1)
    {
        left_start -= 1;
        right_start -= 1;
    }
    let mut left_end = left.line_index;
    let mut right_end = right.line_index;
    while left_end + 1 < files[left.file_index].lines.len()
        && right_end + 1 < files[right.file_index].lines.len()
        && line_text(files, left.file_index, left_end + 1)
            == line_text(files, right.file_index, right_end + 1)
    {
        left_end += 1;
        right_end += 1;
    }
    if left.file_index == right.file_index && left_start <= right_end && right_start <= left_end {
        return None;
    }
    let normalized_lines = files[left.file_index].lines[left_start..=left_end]
        .iter()
        .map(|line| line.normalized_text.clone())
        .collect::<Vec<_>>();
    Some(CandidateBlock {
        normalized_lines,
        left_occurrence: occurrence_for(files, left.file_index, left_start, left_end),
        right_occurrence: occurrence_for(files, right.file_index, right_start, right_end),
    })
}

fn occurrence_for(
    files: &[ProcessedFile],
    file_index: usize,
    start_index: usize,
    end_index: usize,
) -> OccurrenceKey {
    let lines = &files[file_index].lines;
    let file_path = files[file_index].source.display_path.clone();
    OccurrenceKey {
        file_path_key: format_path(&file_path),
        file_path,
        start_line: lines[start_index].line_number,
        end_line: lines[end_index].line_number,
    }
}

fn line_at(files: &[ProcessedFile], line_ref: LineRef) -> &LineEntry {
    &files[line_ref.file_index].lines[line_ref.line_index]
}

fn line_text(files: &[ProcessedFile], file_index: usize, line_index: usize) -> &str {
    &files[file_index].lines[line_index].normalized_text
}

fn compare_duplicate_blocks(left: &DuplicateBlock, right: &DuplicateBlock) -> Ordering {
    right
        .weight
        .cmp(&left.weight)
        .then_with(|| right.line_count().cmp(&left.line_count()))
        .then_with(|| right.character_count().cmp(&left.character_count()))
        .then_with(|| first_occurrence_key(left).cmp(&first_occurrence_key(right)))
        .then_with(|| first_occurrence_start_line(left).cmp(&first_occurrence_start_line(right)))
        .then_with(|| normalized_block_text(left).cmp(&normalized_block_text(right)))
}

fn first_occurrence_key(block: &DuplicateBlock) -> String {
    block
        .occurrences
        .first()
        .map(|occurrence| format_path(&occurrence.file_path))
        .unwrap_or_default()
}

fn first_occurrence_start_line(block: &DuplicateBlock) -> usize {
    block
        .occurrences
        .first()
        .map(|occurrence| occurrence.start_line)
        .unwrap_or_default()
}

fn normalized_block_text(block: &DuplicateBlock) -> String {
    block.normalized_lines.join("\n")
}

#[cfg(test)]
mod tests {
    use crate::language::hash_normalized_line;
    use crate::model::{LineEntry, ProcessedFile, SourceFile};

    use super::*;

    fn processed_file(path: &str, extension: &str, lines: &[(&str, LineStatus)]) -> ProcessedFile {
        let line_entries = lines
            .iter()
            .enumerate()
            .map(|(index, (text, status))| LineEntry {
                file_path: PathBuf::from(path),
                line_number: index + 1,
                normalized_text: (*text).to_string(),
                hash: hash_normalized_line(text),
                status: *status,
            })
            .collect();
        ProcessedFile {
            source: SourceFile {
                path: PathBuf::from(path),
                display_path: PathBuf::from(path),
                extension: extension.to_string(),
            },
            lines: line_entries,
        }
    }

    #[test]
    fn groups_three_occurrences_of_the_same_block() {
        let files = vec![
            processed_file(
                "a.ts",
                "ts",
                &[
                    ("const value = one;", LineStatus::Comparison),
                    ("return value;", LineStatus::Comparison),
                ],
            ),
            processed_file(
                "b.ts",
                "ts",
                &[
                    ("const value = one;", LineStatus::Comparison),
                    ("return value;", LineStatus::Comparison),
                ],
            ),
            processed_file(
                "c.ts",
                "ts",
                &[
                    ("const value = one;", LineStatus::Comparison),
                    ("return value;", LineStatus::Comparison),
                ],
            ),
        ];
        let blocks = detect_duplicate_blocks(&files);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].occurrences.len(), 3);
        assert_eq!(
            blocks[0].normalized_lines,
            ["const value = one;", "return value;"]
        );
    }

    #[test]
    fn ignores_single_line_duplicates_that_are_only_block_only_lines() {
        let files = vec![
            processed_file("a.ts", "ts", &[("}", LineStatus::BlockOnly)]),
            processed_file("b.ts", "ts", &[("}", LineStatus::BlockOnly)]),
        ];
        let blocks = detect_duplicate_blocks(&files);
        assert!(blocks.is_empty());
    }

    #[test]
    fn includes_block_only_lines_inside_larger_duplicate_blocks() {
        let files = vec![
            processed_file(
                "a.ts",
                "ts",
                &[
                    ("if (ready) {", LineStatus::Comparison),
                    ("}", LineStatus::BlockOnly),
                    ("return value;", LineStatus::Comparison),
                ],
            ),
            processed_file(
                "b.ts",
                "ts",
                &[
                    ("if (ready) {", LineStatus::Comparison),
                    ("}", LineStatus::BlockOnly),
                    ("return value;", LineStatus::Comparison),
                ],
            ),
        ];
        let blocks = detect_duplicate_blocks(&files);
        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0].normalized_lines,
            ["if (ready) {", "}", "return value;"]
        );
    }

    #[test]
    fn rejects_overlapping_duplicate_ranges_in_the_same_file() {
        let files = vec![processed_file(
            "a.ts",
            "ts",
            &[
                ("const value = one;", LineStatus::Comparison),
                ("const value = one;", LineStatus::Comparison),
                ("const value = one;", LineStatus::Comparison),
            ],
        )];
        let blocks = detect_duplicate_blocks(&files);
        assert!(!blocks.iter().any(|block| {
            block.normalized_lines == ["const value = one;", "const value = one;"]
                && block
                    .occurrences
                    .iter()
                    .any(|occurrence| occurrence.start_line == 1)
                && block
                    .occurrences
                    .iter()
                    .any(|occurrence| occurrence.start_line == 2)
        }));
    }
}
