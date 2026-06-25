use std::error::Error;
use std::fmt;
use std::io;
use std::path::Path;

use crate::paths::format_path;

pub type Result<T> = std::result::Result<T, CodeM8Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeM8Error {
    message: String,
    show_help: bool,
}

impl CodeM8Error {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            show_help: false,
        }
    }

    #[must_use]
    pub fn with_help(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            show_help: true,
        }
    }

    #[must_use]
    pub fn io(path: &Path, action: &str, error: &io::Error) -> Self {
        Self::new(format!("could not {action} {}: {error}", format_path(path)))
    }

    #[must_use]
    pub const fn should_show_help(&self) -> bool {
        self.show_help
    }
}

impl fmt::Display for CodeM8Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for CodeM8Error {}
