use std::collections::HashMap;
use std::sync::OnceLock;

use crate::model::LineStatus;

#[derive(Debug, Clone, Copy)]
pub struct LanguageLinePattern {
    pub language_name: &'static str,
    pub extensions: &'static [&'static str],
    pub block_only_lines: &'static [&'static str],
}

pub const LANGUAGE_PATTERNS: &[LanguageLinePattern] = &[
    LanguageLinePattern {
        language_name: "TypeScript / JavaScript",
        extensions: &["ts", "tsx", "js", "jsx", "mjs", "cjs"],
        block_only_lines: &[
            "(", ")", "{", "}", "[", "]", ");", "];", "};", ")};", "}),", "});",
        ],
    },
    LanguageLinePattern {
        language_name: "Rust",
        extensions: &["rs"],
        block_only_lines: &["{", "}", "(", ")", "[", "]", ");", "];"],
    },
    LanguageLinePattern {
        language_name: "C / C++ / Objective-C",
        extensions: &["c", "h", "cpp", "hpp", "cc", "hh", "cxx", "hxx", "m", "mm"],
        block_only_lines: &[
            "{", "}", "(", ")", "[", "]", ");", "];", "};", "#endif", "#else",
        ],
    },
    LanguageLinePattern {
        language_name: "C#",
        extensions: &["cs"],
        block_only_lines: &[
            "{",
            "}",
            "(",
            ")",
            "[",
            "]",
            ");",
            "];",
            "};",
            "#endregion",
            "#else",
            "#endif",
        ],
    },
    LanguageLinePattern {
        language_name: "Java / Kotlin / Scala",
        extensions: &["java", "kt", "kts", "scala", "sc"],
        block_only_lines: &["{", "}", "(", ")", "[", "]", ");", "];", "};"],
    },
    LanguageLinePattern {
        language_name: "Go",
        extensions: &["go"],
        block_only_lines: &["{", "}", "(", ")", "[", "]"],
    },
    LanguageLinePattern {
        language_name: "Python",
        extensions: &["py", "pyw"],
        block_only_lines: &["(", ")", "[", "]", "{", "}"],
    },
    LanguageLinePattern {
        language_name: "Ruby",
        extensions: &["rb"],
        block_only_lines: &["(", ")", "[", "]", "{", "}", "end"],
    },
    LanguageLinePattern {
        language_name: "PHP",
        extensions: &["php", "phtml"],
        block_only_lines: &["{", "}", "(", ")", "[", "]", ");", "];", "};", "?>"],
    },
    LanguageLinePattern {
        language_name: "Swift",
        extensions: &["swift"],
        block_only_lines: &["{", "}", "(", ")", "[", "]", ");", "];"],
    },
    LanguageLinePattern {
        language_name: "Shell",
        extensions: &["sh", "bash", "zsh", "fish"],
        block_only_lines: &["then", "do", "done", "fi", "else", "{", "}"],
    },
    LanguageLinePattern {
        language_name: "PowerShell",
        extensions: &["ps1", "psm1", "psd1"],
        block_only_lines: &["{", "}", "(", ")", "[", "]", ");"],
    },
    LanguageLinePattern {
        language_name: "HTML / XML",
        extensions: &["html", "htm", "xml", "xhtml", "svg"],
        block_only_lines: &[
            ">",
            "/>",
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
        block_only_lines: &["{", "}", ");"],
    },
    LanguageLinePattern {
        language_name: "SQL",
        extensions: &["sql"],
        block_only_lines: &["(", ")", ");", ";", "BEGIN", "END"],
    },
    LanguageLinePattern {
        language_name: "YAML / JSON / TOML",
        extensions: &["yaml", "yml", "json", "toml"],
        block_only_lines: &["{", "}", "[", "]", "},", "],"],
    },
];

#[derive(Debug)]
struct BlockOnlyRegistry {
    by_extension: HashMap<&'static str, HashMap<u128, Vec<&'static str>>>,
}

static BLOCK_ONLY_REGISTRY: OnceLock<BlockOnlyRegistry> = OnceLock::new();

pub fn hash_normalized_line(line: &str) -> u128 {
    xxhash_rust::xxh3::xxh3_128(line.as_bytes())
}

pub fn classify_line(extension: &str, normalized_line: &str, hash: u128) -> LineStatus {
    let extension = extension.to_ascii_lowercase();
    let Some(patterns_by_hash) = registry().by_extension.get(extension.as_str()) else {
        return LineStatus::Comparison;
    };
    let Some(patterns) = patterns_by_hash.get(&hash) else {
        return LineStatus::Comparison;
    };
    if patterns.contains(&normalized_line) {
        LineStatus::BlockOnly
    } else {
        LineStatus::Comparison
    }
}

fn registry() -> &'static BlockOnlyRegistry {
    BLOCK_ONLY_REGISTRY.get_or_init(|| {
        let mut by_extension: HashMap<&'static str, HashMap<u128, Vec<&'static str>>> =
            HashMap::new();
        for language in LANGUAGE_PATTERNS {
            for extension in language.extensions {
                register_block_only_lines(
                    by_extension.entry(extension).or_default(),
                    language.block_only_lines,
                );
            }
        }
        BlockOnlyRegistry { by_extension }
    })
}

fn register_block_only_lines(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigns_block_only_status_from_extension_specific_registry() {
        let hash = hash_normalized_line("}");
        assert_eq!(classify_line("ts", "}", hash), LineStatus::BlockOnly);
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
}
