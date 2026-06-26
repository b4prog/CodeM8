use std::path::{Path, PathBuf};

mod explicit;
mod git;
mod recursive;

pub(crate) use git::changed_files_against_origin;

use crate::error::Result;
use crate::model::SourceFile;
use crate::paths::format_path;

/// Discovers source files that match the selected extensions.
///
/// # Errors
///
/// Returns an error when explicit files are invalid or when walking the file
/// tree fails.
pub fn discover_source_files(
    current_dir: &Path,
    extensions: &[String],
    explicit_files: Option<&[PathBuf]>,
) -> Result<Vec<SourceFile>> {
    let mut source_files = if let Some(files) = explicit_files {
        explicit::discover_explicit_files(current_dir, extensions, files)?
    } else {
        recursive::discover_recursive_files(current_dir, extensions)?
    };
    source_files.sort_by(|left, right| {
        format_path(&left.display_path).cmp(&format_path(&right.display_path))
    });
    Ok(source_files)
}

fn selected_extension(path: &Path, extensions: &[String]) -> Option<String> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    extensions
        .iter()
        .any(|selected| selected.eq_ignore_ascii_case(&extension))
        .then_some(extension)
}
