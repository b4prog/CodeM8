use std::collections::HashMap;
use std::sync::OnceLock;

use crate::model::LineStatus;

#[derive(Debug, Clone, Copy)]
pub struct LanguageLinePattern {
    pub language_name: &'static str,
    pub extensions: &'static [&'static str],
    pub duplicate_mitigation_pattern: &'static [char],
    pub duplicate_mitigation_lines: &'static [&'static str],
}

pub const LANGUAGE_PATTERNS: &[LanguageLinePattern] = &[
    LanguageLinePattern {
        language_name: "TypeScript / JavaScript",
        extensions: &["ts", "tsx", "js", "jsx", "mjs", "cjs"],
        duplicate_mitigation_pattern: &['(', ')', '{', '}', '[', ']', ';', ',', '?', ':', '<', '>'],
        duplicate_mitigation_lines: &[],
    },
    LanguageLinePattern {
        language_name: "Rust",
        extensions: &["rs"],
        duplicate_mitigation_pattern: &['(', ')', '{', '}', '[', ']', ';', ',', '?', ':', '<', '>'],
        duplicate_mitigation_lines: &[".into_iter()", "///"],
    },
    LanguageLinePattern {
        language_name: "C / C++ / Objective-C",
        extensions: &["c", "h", "cpp", "hpp", "cc", "hh", "cxx", "hxx", "m", "mm"],
        duplicate_mitigation_pattern: &['(', ')', '{', '}', '[', ']', ';', ',', '?', ':', '<', '>'],
        duplicate_mitigation_lines: &["#endif", "#else"],
    },
    LanguageLinePattern {
        language_name: "C#",
        extensions: &["cs"],
        duplicate_mitigation_pattern: &['(', ')', '{', '}', '[', ']', ';', ',', '?', ':', '<', '>'],
        duplicate_mitigation_lines: &["#endregion", "#else", "#endif"],
    },
    LanguageLinePattern {
        language_name: "Java / Kotlin / Scala",
        extensions: &["java", "kt", "kts", "scala", "sc"],
        duplicate_mitigation_pattern: &['(', ')', '{', '}', '[', ']', ';', ',', '?', ':', '<', '>'],
        duplicate_mitigation_lines: &[],
    },
    LanguageLinePattern {
        language_name: "Go",
        extensions: &["go"],
        duplicate_mitigation_pattern: &['(', ')', '{', '}', '[', ']', ';', ',', ':'],
        duplicate_mitigation_lines: &[],
    },
    LanguageLinePattern {
        language_name: "Python",
        extensions: &["py", "pyw"],
        duplicate_mitigation_pattern: &['(', ')', '{', '}', '[', ']', ';', ',', ':'],
        duplicate_mitigation_lines: &[],
    },
    LanguageLinePattern {
        language_name: "Ruby",
        extensions: &["rb"],
        duplicate_mitigation_pattern: &['(', ')', '{', '}', '[', ']', ';', ',', '?', ':'],
        duplicate_mitigation_lines: &["end"],
    },
    LanguageLinePattern {
        language_name: "PHP",
        extensions: &["php", "phtml"],
        duplicate_mitigation_pattern: &[
            '(', ')', '{', '}', '[', ']', ';', ',', '?', ':', '<', '>', '/',
        ],
        duplicate_mitigation_lines: &[],
    },
    LanguageLinePattern {
        language_name: "Swift",
        extensions: &["swift"],
        duplicate_mitigation_pattern: &['(', ')', '{', '}', '[', ']', ';', ',', '?', ':', '<', '>'],
        duplicate_mitigation_lines: &[],
    },
    LanguageLinePattern {
        language_name: "Shell",
        extensions: &["sh", "bash", "zsh", "fish"],
        duplicate_mitigation_pattern: &['(', ')', '{', '}', '[', ']', ';', '&', '|'],
        duplicate_mitigation_lines: &["then", "do", "done", "fi", "else"],
    },
    LanguageLinePattern {
        language_name: "PowerShell",
        extensions: &["ps1", "psm1", "psd1"],
        duplicate_mitigation_pattern: &['(', ')', '{', '}', '[', ']', ';', ',', '?', ':', '|'],
        duplicate_mitigation_lines: &[],
    },
    LanguageLinePattern {
        language_name: "HTML / XML",
        extensions: &["html", "htm", "xml", "xhtml", "svg"],
        duplicate_mitigation_pattern: &['<', '>', '/'],
        duplicate_mitigation_lines: &[
            "</div>",
            "</span>",
            "</section>",
            "</article>",
            "</body>",
            "</html>",
        ],
    },
    LanguageLinePattern {
        language_name: "CSS / SCSS / Sass / Less",
        extensions: &["css", "scss", "sass", "less"],
        duplicate_mitigation_pattern: &['(', ')', '{', '}', '[', ']', ';', ',', ':'],
        duplicate_mitigation_lines: &[],
    },
    LanguageLinePattern {
        language_name: "SQL",
        extensions: &["sql"],
        duplicate_mitigation_pattern: &['(', ')', ';', ',', ':'],
        duplicate_mitigation_lines: &["BEGIN", "END"],
    },
    LanguageLinePattern {
        language_name: "YAML / JSON / TOML",
        extensions: &["yaml", "yml", "json", "toml"],
        duplicate_mitigation_pattern: &['(', ')', '{', '}', '[', ']', ';', ',', '?', ':', '<', '>'],
        duplicate_mitigation_lines: &["jobs:", "on:"],
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
            }
        }
        DuplicateMitigationLineRegistry { by_extension }
    })
}

impl DuplicateMitigationPatterns {
    fn matches_line(&self, normalized_line: &str, hash: u128) -> bool {
        self.matches_registered_line(normalized_line, hash)
            || matches_duplicate_mitigation_pattern(normalized_line, &self.character_pattern)
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

fn matches_duplicate_mitigation_pattern(line: &str, character_pattern: &[char]) -> bool {
    !character_pattern.is_empty()
        && line
            .chars()
            .all(|character| character.is_whitespace() || character_pattern.contains(&character))
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
}
