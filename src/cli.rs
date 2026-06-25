use std::path::PathBuf;

use crate::error::{CodeM8Error, Result};

const DEFAULT_FILE_EXTENSIONS: &[&str] = &["ts"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliConfig {
    pub report_duplicate: bool,
    pub file_extensions: Vec<String>,
    pub files: Option<Vec<PathBuf>>,
}

pub fn parse_args<I, S>(args: I) -> Result<CliConfig>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut report_duplicate = false;
    let mut file_extensions = None;
    let mut files = None;
    for arg in args {
        let arg = arg.into();
        if arg == "--report-duplicate" {
            report_duplicate = true;
        } else if let Some(value) = arg
            .strip_prefix("-file-extension=")
            .or_else(|| arg.strip_prefix("--file-extension="))
        {
            if file_extensions.is_some() {
                return Err(CodeM8Error::new(
                    "file extensions were provided more than once",
                ));
            }
            file_extensions = Some(parse_file_extensions(value)?);
        } else if let Some(value) = arg
            .strip_prefix("-files=")
            .or_else(|| arg.strip_prefix("--files="))
        {
            if files.is_some() {
                return Err(CodeM8Error::new(
                    "explicit files were provided more than once",
                ));
            }
            files = Some(parse_file_list(value)?);
        } else {
            return Err(CodeM8Error::new(format!("unknown argument: {arg}")));
        }
    }
    if !report_duplicate {
        return Err(CodeM8Error::new(
            "no report switch provided; pass --report-duplicate",
        ));
    }
    Ok(CliConfig {
        report_duplicate,
        file_extensions: file_extensions.unwrap_or_else(|| {
            DEFAULT_FILE_EXTENSIONS
                .iter()
                .map(|extension| extension.to_string())
                .collect()
        }),
        files,
    })
}

pub fn parse_file_extensions(value: &str) -> Result<Vec<String>> {
    let mut extensions = Vec::new();
    for raw_extension in value.split(',') {
        let extension = raw_extension.trim();
        if extension.is_empty() {
            return Err(CodeM8Error::new("file extension values must not be empty"));
        }
        if extension.starts_with('.') {
            return Err(CodeM8Error::new(format!(
                "file extensions must not start with a dot: {extension}"
            )));
        }
        if extension.contains('/') || extension.contains('\\') {
            return Err(CodeM8Error::new(format!(
                "file extensions must not contain path separators: {extension}"
            )));
        }
        let extension = extension.to_ascii_lowercase();
        if !extensions.contains(&extension) {
            extensions.push(extension);
        }
    }
    if extensions.is_empty() {
        return Err(CodeM8Error::new("at least one file extension is required"));
    }
    Ok(extensions)
}

pub fn parse_file_list(value: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for raw_file in value.split(',') {
        let file = raw_file.trim();
        if file.is_empty() {
            return Err(CodeM8Error::new("file path values must not be empty"));
        }
        files.push(PathBuf::from(file));
    }
    if files.is_empty() {
        return Err(CodeM8Error::new("at least one explicit file is required"));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_duplicate_report_config() {
        let config = parse_args(["--report-duplicate"]).expect("config parses");
        assert!(config.report_duplicate);
        assert_eq!(config.file_extensions, ["ts"]);
        assert_eq!(config.files, None);
    }

    #[test]
    fn parses_extensions_case_insensitively_and_trims_whitespace() {
        let extensions = parse_file_extensions(" ts, JS ,tsx,ts ").expect("extensions parse");
        assert_eq!(extensions, ["ts", "js", "tsx"]);
    }

    #[test]
    fn rejects_empty_extensions() {
        let error = parse_file_extensions("ts,,js").expect_err("empty extension fails");
        assert!(error.to_string().contains("must not be empty"));
    }

    #[test]
    fn rejects_extensions_with_leading_dot() {
        let error = parse_file_extensions(".ts").expect_err("dot-prefixed extension fails");
        assert!(error.to_string().contains("must not start with a dot"));
    }

    #[test]
    fn rejects_missing_report_switch() {
        let error = parse_args(["-file-extension=rs"]).expect_err("missing report fails");
        assert!(error.to_string().contains("no report switch provided"));
    }

    #[test]
    fn parses_explicit_file_list() {
        let files = parse_file_list("src/a.ts, ./src/b.ts").expect("files parse");
        assert_eq!(
            files,
            [PathBuf::from("src/a.ts"), PathBuf::from("./src/b.ts")]
        );
    }
}
