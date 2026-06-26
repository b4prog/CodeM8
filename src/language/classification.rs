use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;

use crate::model::LineStatus;

use super::patterns::LANGUAGE_PATTERNS;

#[derive(Debug)]
struct DuplicateMitigationLineRegistry {
    by_extension: HashMap<&'static str, DuplicateMitigationPatterns>,
}

#[derive(Debug, Default)]
struct DuplicateMitigationPatterns {
    lines_by_hash: HashMap<u128, Vec<&'static str>>,
    character_pattern: Vec<char>,
    regexps: Vec<Regex>,
}

static DUPLICATE_MITIGATION_LINE_REGISTRY: OnceLock<DuplicateMitigationLineRegistry> =
    OnceLock::new();

#[must_use]
pub fn hash_normalized_line(line: &str) -> u128 {
    xxhash_rust::xxh3::xxh3_128(line.as_bytes())
}

#[must_use]
pub fn classify_line(extension: &str, normalized_line: &str, hash: u128) -> LineStatus {
    let extension = extension.to_ascii_lowercase();
    let Some(patterns) = registry().by_extension.get(extension.as_str()) else {
        return LineStatus::Comparison;
    };
    if patterns.matches_line(normalized_line, hash) {
        LineStatus::BlockOnly
    } else {
        LineStatus::Comparison
    }
}

fn registry() -> &'static DuplicateMitigationLineRegistry {
    DUPLICATE_MITIGATION_LINE_REGISTRY.get_or_init(|| {
        let mut by_extension: HashMap<&'static str, DuplicateMitigationPatterns> = HashMap::new();
        for language in LANGUAGE_PATTERNS {
            for extension in language.language.extensions {
                let patterns = by_extension.entry(extension).or_default();
                register_duplicate_mitigation_lines(
                    &mut patterns.lines_by_hash,
                    language.duplicate_mitigation_lines,
                );
                register_duplicate_mitigation_pattern(
                    &mut patterns.character_pattern,
                    language.duplicate_mitigation_pattern,
                );
                register_duplicate_mitigation_regexps(
                    &mut patterns.regexps,
                    language.duplicate_mitigation_regexps,
                );
            }
        }
        DuplicateMitigationLineRegistry { by_extension }
    })
}

impl DuplicateMitigationPatterns {
    fn matches_line(&self, normalized_line: &str, hash: u128) -> bool {
        self.matches_registered_line(normalized_line, hash)
            || matches_duplicate_mitigation_pattern(normalized_line, &self.character_pattern)
            || matches_duplicate_mitigation_regexps(normalized_line, &self.regexps)
    }

    fn matches_registered_line(&self, normalized_line: &str, hash: u128) -> bool {
        self.lines_by_hash
            .get(&hash)
            .is_some_and(|patterns| patterns.contains(&normalized_line))
    }
}

fn register_duplicate_mitigation_lines(
    patterns_by_hash: &mut HashMap<u128, Vec<&'static str>>,
    lines: &'static [&'static str],
) {
    for &line in lines {
        patterns_by_hash
            .entry(hash_normalized_line(line))
            .or_default()
            .push(line);
    }
}

fn register_duplicate_mitigation_pattern(
    character_pattern: &mut Vec<char>,
    characters: &'static [char],
) {
    for &character in characters {
        if !character_pattern.contains(&character) {
            character_pattern.push(character);
        }
    }
}

fn register_duplicate_mitigation_regexps(
    regexps: &mut Vec<Regex>,
    patterns: &'static [&'static str],
) {
    for &pattern in patterns {
        if !regexps.iter().any(|regexp| regexp.as_str() == pattern) {
            regexps.push(Regex::new(pattern).expect("duplicate mitigation regexp must compile"));
        }
    }
}

fn matches_duplicate_mitigation_pattern(line: &str, character_pattern: &[char]) -> bool {
    !character_pattern.is_empty()
        && line
            .chars()
            .all(|character| character.is_whitespace() || character_pattern.contains(&character))
}

fn matches_duplicate_mitigation_regexps(line: &str, regexps: &[Regex]) -> bool {
    regexps.iter().any(|regexp| {
        regexp
            .find(line)
            .is_some_and(|matched| matched.start() == 0 && matched.end() == line.len())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigns_block_only_status_from_extension_specific_line_registry() {
        let line = ".into_iter()";
        let hash = hash_normalized_line(line);
        assert_eq!(classify_line("rs", line, hash), LineStatus::BlockOnly);
    }

    #[test]
    fn assigns_block_only_status_for_rust_assert_macro_openers() {
        for line in ["assert!(", "assert_eq!("] {
            let hash = hash_normalized_line(line);
            assert_eq!(classify_line("rs", line, hash), LineStatus::BlockOnly);
        }
    }

    #[test]
    fn assigns_comparison_status_for_meaningful_lines() {
        let line = "const value = computeValue(input);";
        let hash = hash_normalized_line(line);
        assert_eq!(classify_line("ts", line, hash), LineStatus::Comparison);
    }

    #[test]
    fn verifies_text_after_hash_lookup() {
        let hash = hash_normalized_line("}");
        assert_eq!(
            classify_line("ts", "not-a-brace", hash),
            LineStatus::Comparison
        );
    }

    #[test]
    fn assigns_block_only_status_from_character_pattern() {
        let line = "} \t);";
        let hash = hash_normalized_line(line);
        assert_eq!(classify_line("ts", line, hash), LineStatus::BlockOnly);
    }

    #[test]
    fn assigns_block_only_status_from_regexps() {
        let line = ".update()";
        let hash = hash_normalized_line(line);
        assert_eq!(classify_line("rs", line, hash), LineStatus::BlockOnly);
    }

    #[test]
    fn regexps_must_match_the_full_line() {
        let line = ".update()?.await";
        let hash = hash_normalized_line(line);
        assert_eq!(classify_line("rs", line, hash), LineStatus::Comparison);
    }

    #[test]
    fn assigns_block_only_status_for_typescript_codegen_lines() {
        let lines = [
            "// @ts-nocheck",
            "/* eslint-disable */",
            "errors: DeleteViewsError[]",
            "__typename: 'DeleteViewsResponse'",
        ];
        for line in lines {
            let hash = hash_normalized_line(line);
            assert_eq!(classify_line("ts", line, hash), LineStatus::BlockOnly);
        }
    }

    #[test]
    fn assigns_block_only_status_for_yaml_lines() {
        let line = "jobs:";
        let hash = hash_normalized_line(line);
        assert_eq!(classify_line("yaml", line, hash), LineStatus::BlockOnly);
    }

    #[test]
    fn assigns_comparison_status_for_json_lines() {
        let line = "}";
        let hash = hash_normalized_line(line);
        assert_eq!(classify_line("json", line, hash), LineStatus::Comparison);
    }

    #[test]
    fn ignores_character_pattern_for_unknown_extensions() {
        let line = "});";
        let hash = hash_normalized_line(line);
        assert_eq!(classify_line("unknown", line, hash), LineStatus::Comparison);
    }

    #[test]
    fn empty_character_pattern_does_not_match() {
        assert!(!matches_duplicate_mitigation_pattern("}", &[]));
    }
}
