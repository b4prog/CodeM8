#[derive(Debug, Clone, Copy)]
pub(super) struct Language {
    pub(super) language_name: &'static str,
    pub(super) extensions: &'static [&'static str],
}

pub(super) static BASH: Language = Language {
    language_name: "Bash",
    extensions: &["bash"],
};

pub(super) static C: Language = Language {
    language_name: "C",
    extensions: &["c", "h"],
};

pub(super) static C_SHARP: Language = Language {
    language_name: "C#",
    extensions: &["cs"],
};

pub(super) static C_PLUS_PLUS: Language = Language {
    language_name: "C++",
    extensions: &["cpp", "hpp", "cc", "hh", "cxx", "hxx"],
};

pub(super) static CSS: Language = Language {
    language_name: "CSS",
    extensions: &["css"],
};

pub(super) static FISH: Language = Language {
    language_name: "Fish",
    extensions: &["fish"],
};

pub(super) static GO: Language = Language {
    language_name: "Go",
    extensions: &["go"],
};

pub(super) static HTML: Language = Language {
    language_name: "HTML",
    extensions: &["html", "htm"],
};

pub(super) static JAVA: Language = Language {
    language_name: "Java",
    extensions: &["java"],
};

pub(super) static JAVASCRIPT: Language = Language {
    language_name: "JavaScript",
    extensions: &["js", "jsx", "mjs", "cjs"],
};

pub(super) static KOTLIN: Language = Language {
    language_name: "Kotlin",
    extensions: &["kt", "kts"],
};

pub(super) static LESS: Language = Language {
    language_name: "Less",
    extensions: &["less"],
};

pub(super) static OBJECTIVE_C: Language = Language {
    language_name: "Objective-C",
    extensions: &["m", "mm"],
};

pub(super) static PHP: Language = Language {
    language_name: "PHP",
    extensions: &["php", "phtml"],
};

pub(super) static POWERSHELL: Language = Language {
    language_name: "PowerShell",
    extensions: &["ps1", "psm1", "psd1"],
};

pub(super) static PYTHON: Language = Language {
    language_name: "Python",
    extensions: &["py", "pyw"],
};

pub(super) static RUBY: Language = Language {
    language_name: "Ruby",
    extensions: &["rb"],
};

pub(super) static RUST: Language = Language {
    language_name: "Rust",
    extensions: &["rs"],
};

pub(super) static SASS: Language = Language {
    language_name: "Sass",
    extensions: &["sass"],
};

pub(super) static SCALA: Language = Language {
    language_name: "Scala",
    extensions: &["scala", "sc"],
};

pub(super) static SCSS: Language = Language {
    language_name: "SCSS",
    extensions: &["scss"],
};

pub(super) static SHELL: Language = Language {
    language_name: "Shell",
    extensions: &["sh"],
};

pub(super) static SQL: Language = Language {
    language_name: "SQL",
    extensions: &["sql"],
};

pub(super) static SWIFT: Language = Language {
    language_name: "Swift",
    extensions: &["swift"],
};

pub(super) static TYPESCRIPT: Language = Language {
    language_name: "TypeScript",
    extensions: &["ts", "tsx"],
};

pub(super) static XML: Language = Language {
    language_name: "XML",
    extensions: &["xml", "xhtml", "svg"],
};

pub(super) static YAML: Language = Language {
    language_name: "YAML",
    extensions: &["yaml", "yml"],
};

pub(super) static ZSH: Language = Language {
    language_name: "Zsh",
    extensions: &["zsh"],
};

pub(super) const LANGUAGES: &[&Language] = &[
    &BASH,
    &C,
    &C_SHARP,
    &C_PLUS_PLUS,
    &CSS,
    &FISH,
    &GO,
    &HTML,
    &JAVA,
    &JAVASCRIPT,
    &KOTLIN,
    &LESS,
    &OBJECTIVE_C,
    &PHP,
    &POWERSHELL,
    &PYTHON,
    &RUBY,
    &RUST,
    &SASS,
    &SCALA,
    &SCSS,
    &SHELL,
    &SQL,
    &SWIFT,
    &TYPESCRIPT,
    &XML,
    &YAML,
    &ZSH,
];

#[must_use]
pub fn supported_file_extensions() -> Vec<String> {
    let mut extensions = Vec::new();
    for language in LANGUAGES {
        debug_assert!(!language.language_name.is_empty());
        for &extension in language.extensions {
            if !extensions.iter().any(|selected| selected == extension) {
                extensions.push(extension.to_string());
            }
        }
    }
    extensions
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn collects_supported_file_extensions_from_language_registry() {
        let extensions = supported_file_extensions();
        for language in LANGUAGES {
            for extension in language.extensions {
                assert!(extensions.iter().any(|selected| selected == extension));
            }
        }
    }

    #[test]
    fn languages_are_sorted_by_name() {
        for pair in LANGUAGES.windows(2) {
            assert!(
                pair[0].language_name.to_ascii_lowercase()
                    <= pair[1].language_name.to_ascii_lowercase()
            );
        }
    }

    #[test]
    fn languages_use_unique_extensions() {
        let mut languages_by_extension = HashMap::new();
        for language in LANGUAGES {
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
