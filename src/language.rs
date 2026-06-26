use std::collections::HashMap;
use std::sync::OnceLock;

use crate::model::LineStatus;
use regex::Regex;

#[derive(Debug, Clone, Copy)]
pub struct LanguageLinePattern {
    pub language_name: &'static str,
    pub extensions: &'static [&'static str],
    pub duplicate_mitigation_pattern: &'static [char],
    pub duplicate_mitigation_lines: &'static [&'static str],
    pub duplicate_mitigation_regexps: &'static [&'static str],
}

pub const LANGUAGE_PATTERNS: &[LanguageLinePattern] = &[
    LanguageLinePattern {
        language_name: "Bash",
        extensions: &["bash"],
        duplicate_mitigation_pattern: &['&', '(', ')', ';', '[', ']', '{', '|', '}'],
        duplicate_mitigation_lines: &["do", "done", "else", "fi", "then"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "C",
        extensions: &["c", "h"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &["#else", "#endif"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "C#",
        extensions: &["cs"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &["#else", "#endif", "#endregion"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "C++",
        extensions: &["cpp", "hpp", "cc", "hh", "cxx", "hxx"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &["#else", "#endif"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "CSS",
        extensions: &["css"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "Fish",
        extensions: &["fish"],
        duplicate_mitigation_pattern: &['&', '(', ')', ';', '[', ']', '{', '|', '}'],
        duplicate_mitigation_lines: &["else", "end"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "Go",
        extensions: &["go"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "HTML",
        extensions: &["html", "htm"],
        duplicate_mitigation_pattern: &['/', '<', '>'],
        duplicate_mitigation_lines: &[
            "</article>",
            "</body>",
            "</div>",
            "</html>",
            "</section>",
            "</span>",
        ],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "Java",
        extensions: &["java"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "JavaScript",
        extensions: &["js", "jsx", "mjs", "cjs"],
        duplicate_mitigation_pattern: &[
            '&', '(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '|', '}',
        ],
        duplicate_mitigation_lines: &["// @ts-nocheck"],
        duplicate_mitigation_regexps: &[
            // Excludes single-line block comments used by generated files and tooling. Example: /* eslint-disable */
            r"^/\*.*\*/$",
            // Excludes generated interface field declarations. Example: errors: InvalidInputError[]
            r"^[A-Za-z_$][A-Za-z0-9_$]*\??:\s*(?:Scalars\['[A-Za-z]+'\]|[A-Z][A-Za-z0-9_$]*(?:\[\])?|[a-z]+(?:\[\])?|\([^)]*\))(?:\[\])?(?:\s*\|\s*(?:null|number|boolean|string))*[,]?$",
            // Excludes generated GraphQL typename marker fields. Example: __typename: 'User'
            r"^__typename:\s*'[A-Za-z_$][A-Za-z0-9_$]*'[,]?$",
        ],
    },
    LanguageLinePattern {
        language_name: "Kotlin",
        extensions: &["kt", "kts"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "Less",
        extensions: &["less"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "Objective-C",
        extensions: &["m", "mm"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &["#else", "#endif"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "PHP",
        extensions: &["php", "phtml"],
        duplicate_mitigation_pattern: &[
            '(', ')', ',', '/', ':', ';', '<', '>', '?', '[', ']', '{', '}',
        ],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "PowerShell",
        extensions: &["ps1", "psm1", "psd1"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '?', '[', ']', '{', '|', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "Python",
        extensions: &["py", "pyw"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "Ruby",
        extensions: &["rb"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &["end"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "Rust",
        extensions: &["rs"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &["///", "#[test]"],
        duplicate_mitigation_regexps: &[
            // Excludes short path or enum variant fragments. Example: Self::Ready,
            r"^[A-Za-z0-9_]*::?\s*[A-Za-z0-9_]*[,]?$",
            // Excludes bare identifiers with optional punctuation. Example: value,
            r"^[A-Za-z0-9_]+\s*[.,]?$",
            // Excludes simple method or field access lines. Example: .clone()
            r"^\.?\s*[A-Za-z0-9_]+(?:\(\s*\)?)?$",
            // Excludes incomplete let bindings split across lines. Example: let value =
            r"^let\s+(?:mut\s+)?[A-Za-z0-9_]+\s*=$",
            // Excludes simple public struct field declarations. Example: pub name: String,
            r"^pub\s+[A-Za-z0-9_]*\s*:\s*[A-Za-z0-9_]*[,]?$",
            // Excludes single-path use imports. Example: use crate::module;
            r"^use\s+[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*;$",
        ],
    },
    LanguageLinePattern {
        language_name: "Sass",
        extensions: &["sass"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "Scala",
        extensions: &["scala", "sc"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "SCSS",
        extensions: &["scss"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "Shell",
        extensions: &["sh"],
        duplicate_mitigation_pattern: &['&', '(', ')', ';', '[', ']', '{', '|', '}'],
        duplicate_mitigation_lines: &["do", "done", "else", "fi", "then"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "SQL",
        extensions: &["sql"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';'],
        duplicate_mitigation_lines: &["BEGIN", "END"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "Swift",
        extensions: &["swift"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "TypeScript",
        extensions: &["ts", "tsx"],
        duplicate_mitigation_pattern: &[
            '&', '(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '|', '}',
        ],
        duplicate_mitigation_lines: &["// @ts-nocheck"],
        duplicate_mitigation_regexps: &[
            // Excludes single-line block comments used by generated files and tooling. Example: /* eslint-disable */
            r"^/\*.*\*/$",
            // Excludes generated interface field declarations. Example: errors: InvalidInputError[]
            r"^[A-Za-z_$][A-Za-z0-9_$]*\??:\s*(?:Scalars\['[A-Za-z]+'\]|[A-Z][A-Za-z0-9_$]*(?:\[\])?|[a-z]+(?:\[\])?|\([^)]*\))(?:\[\])?(?:\s*\|\s*(?:null|number|boolean|string))*[,]?$",
            // Excludes generated GraphQL typename marker fields. Example: __typename: 'User'
            r"^__typename:\s*'[A-Za-z_$][A-Za-z0-9_$]*'[,]?$",
        ],
    },
    LanguageLinePattern {
        language_name: "XML",
        extensions: &["xml", "xhtml", "svg"],
        duplicate_mitigation_pattern: &['/', '<', '>'],
        duplicate_mitigation_lines: &[
            "</article>",
            "</body>",
            "</div>",
            "</html>",
            "</section>",
            "</span>",
        ],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "YAML",
        extensions: &["yaml", "yml"],
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &["jobs:", "on:"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language_name: "Zsh",
        extensions: &["zsh"],
        duplicate_mitigation_pattern: &['&', '(', ')', ';', '[', ']', '{', '|', '}'],
        duplicate_mitigation_lines: &["do", "done", "else", "fi", "then"],
        duplicate_mitigation_regexps: &[],
    },
];

#[must_use]
pub fn supported_file_extensions() -> Vec<String> {
    let mut extensions = Vec::new();
    for language in LANGUAGE_PATTERNS {
        for &extension in language.extensions {
            if !extensions.iter().any(|selected| selected == extension) {
                extensions.push(extension.to_string());
            }
        }
    }
    extensions
}

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
            for extension in language.extensions {
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

    #[test]
    fn collects_supported_file_extensions_from_language_patterns() {
        let extensions = supported_file_extensions();
        for language in LANGUAGE_PATTERNS {
            for extension in language.extensions {
                assert!(extensions.iter().any(|selected| selected == extension));
            }
        }
    }

    #[test]
    fn language_patterns_are_sorted_by_name() {
        for pair in LANGUAGE_PATTERNS.windows(2) {
            assert!(
                pair[0].language_name.to_ascii_lowercase()
                    <= pair[1].language_name.to_ascii_lowercase()
            );
        }
    }

    #[test]
    fn language_patterns_use_unique_extensions() {
        let mut languages_by_extension = HashMap::new();
        for language in LANGUAGE_PATTERNS {
            for extension in language.extensions {
                let previous = languages_by_extension.insert(extension, language.language_name);
                assert!(
                    previous.is_none(),
                    "{extension} belongs to both {} and {}",
                    previous.unwrap_or_default(),
                    language.language_name
                );
            }
        }
    }
}
