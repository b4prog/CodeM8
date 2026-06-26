use super::registry::{
    Language, BASH, C, CSS, C_PLUS_PLUS, C_SHARP, FISH, GO, HTML, JAVA, JAVASCRIPT, KOTLIN, LESS,
    OBJECTIVE_C, PHP, POWERSHELL, PYTHON, RUBY, RUST, SASS, SCALA, SCSS, SHELL, SQL, SWIFT,
    TYPESCRIPT, XML, YAML, ZSH,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct LanguageLinePattern {
    pub(super) language: &'static Language,
    pub(super) duplicate_mitigation_pattern: &'static [char],
    pub(super) duplicate_mitigation_lines: &'static [&'static str],
    pub(super) duplicate_mitigation_regexps: &'static [&'static str],
}

pub(super) const LANGUAGE_PATTERNS: &[LanguageLinePattern] = &[
    LanguageLinePattern {
        language: &BASH,
        duplicate_mitigation_pattern: &['&', '(', ')', ';', '[', ']', '{', '|', '}'],
        duplicate_mitigation_lines: &["do", "done", "else", "fi", "then"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &C,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &["#else", "#endif"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &C_SHARP,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &["#else", "#endif", "#endregion"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &C_PLUS_PLUS,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &["#else", "#endif"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &CSS,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &FISH,
        duplicate_mitigation_pattern: &['&', '(', ')', ';', '[', ']', '{', '|', '}'],
        duplicate_mitigation_lines: &["else", "end"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &GO,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &HTML,
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
        language: &JAVA,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &JAVASCRIPT,
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
        language: &KOTLIN,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &LESS,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &OBJECTIVE_C,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &["#else", "#endif"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &PHP,
        duplicate_mitigation_pattern: &[
            '(', ')', ',', '/', ':', ';', '<', '>', '?', '[', ']', '{', '}',
        ],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &POWERSHELL,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '?', '[', ']', '{', '|', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &PYTHON,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &RUBY,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &["end"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &RUST,
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
        language: &SASS,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &SCALA,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &SCSS,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &SHELL,
        duplicate_mitigation_pattern: &['&', '(', ')', ';', '[', ']', '{', '|', '}'],
        duplicate_mitigation_lines: &["do", "done", "else", "fi", "then"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &SQL,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';'],
        duplicate_mitigation_lines: &["BEGIN", "END"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &SWIFT,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &[],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &TYPESCRIPT,
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
        language: &XML,
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
        language: &YAML,
        duplicate_mitigation_pattern: &['(', ')', ',', ':', ';', '<', '>', '?', '[', ']', '{', '}'],
        duplicate_mitigation_lines: &["jobs:", "on:"],
        duplicate_mitigation_regexps: &[],
    },
    LanguageLinePattern {
        language: &ZSH,
        duplicate_mitigation_pattern: &['&', '(', ')', ';', '[', ']', '{', '|', '}'],
        duplicate_mitigation_lines: &["do", "done", "else", "fi", "then"],
        duplicate_mitigation_regexps: &[],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_patterns_are_sorted_by_language_name() {
        for pair in LANGUAGE_PATTERNS.windows(2) {
            assert!(
                pair[0].language.language_name.to_ascii_lowercase()
                    <= pair[1].language.language_name.to_ascii_lowercase()
            );
        }
    }
}
