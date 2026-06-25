use std::error::Error;
use std::fmt;
use std::io;
use std::path::Path;

use crate::paths::format_path;

pub type Result<T> = std::result::Result<T, CodeM8Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeM8Error {
    message: String,
}

impl CodeM8Error {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn io(path: &Path, action: &str, error: &io::Error) -> Self {
        Self::new(format!("could not {action} {}: {error}", format_path(path)))
    }
}

impl fmt::Display for CodeM8Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for CodeM8Error {}
