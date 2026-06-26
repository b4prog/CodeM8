use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::selected_extension;
use crate::error::{CodeM8Error, Result};
use crate::model::SourceFile;
use crate::paths::{format_path, normalize_display_path};

pub(super) fn discover_explicit_files(
    current_dir: &Path,
    extensions: &[String],
    files: &[PathBuf],
) -> Result<Vec<SourceFile>> {
    let mut source_files = Vec::new();
    let mut seen_paths = HashSet::new();
    for file in files {
        let absolute_input = file.is_absolute();
        let path = if absolute_input {
            file.clone()
        } else {
            current_dir.join(file)
        };
        let metadata = fs::symlink_metadata(&path).map_err(|_| {
            CodeM8Error::new(format!(
                "explicit file does not exist: {}",
                format_path(file)
            ))
        })?;
        if metadata.file_type().is_symlink() {
            return Err(CodeM8Error::new(format!(
                "explicit file is a symbolic link and will not be followed: {}",
                format_path(file)
            )));
        }
        if metadata.is_dir() {
            return Err(CodeM8Error::new(format!(
                "explicit file is a directory: {}",
                format_path(file)
            )));
        }
        if !metadata.is_file() {
            return Err(CodeM8Error::new(format!(
                "explicit path is not a file: {}",
                format_path(file)
            )));
        }
        let Some(extension) = selected_extension(&path, extensions) else {
            continue;
        };
        let canonical_path = fs::canonicalize(&path)
            .map_err(|error| CodeM8Error::io(&path, "canonicalize explicit file", &error))?;
        if !seen_paths.insert(canonical_path.clone()) {
            continue;
        }
        let display_path = if absolute_input {
            canonical_path
                .strip_prefix(current_dir)
                .map_or_else(|_| normalize_display_path(file), normalize_display_path)
        } else {
            normalize_display_path(file)
        };
        source_files.push(SourceFile {
            path: canonical_path,
            display_path,
            extension,
        });
    }
    Ok(source_files)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir(name: &str) -> PathBuf {
        let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "codem8-discovery-explicit-{name}-{}-{id}",
            std::process::id()
        ));
        if path.exists() {
            fs::remove_dir_all(&path).expect("remove stale test directory");
        }
        fs::create_dir_all(&path).expect("create test directory");
        path
    }

    #[test]
    fn explicit_files_skip_unselected_extensions() {
        let root = temp_dir("skip");
        fs::write(root.join("a.ts"), "").expect("write ts");
        fs::write(root.join("b.js"), "").expect("write js");
        let files = discover_explicit_files(
            &root,
            &["ts".to_string()],
            &[PathBuf::from("a.ts"), PathBuf::from("b.js")],
        )
        .expect("discover");
        assert_eq!(files.len(), 1);
        assert_eq!(format_path(&files[0].display_path), "a.ts");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn explicit_files_deduplicate_resolved_paths() {
        let root = temp_dir("dedup");
        fs::write(root.join("a.ts"), "").expect("write ts");
        let absolute = fs::canonicalize(root.join("a.ts")).expect("canonicalize ts");
        let files = discover_explicit_files(
            &root,
            &["ts".to_string()],
            &[
                PathBuf::from("a.ts"),
                PathBuf::from(".").join("a.ts"),
                absolute.clone(),
            ],
        )
        .expect("discover");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, absolute);
        assert_eq!(format_path(&files[0].display_path), "a.ts");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn explicit_files_reject_directories() {
        let root = temp_dir("directory");
        fs::create_dir_all(root.join("src")).expect("create explicit directory");
        let error = discover_explicit_files(&root, &["ts".to_string()], &[PathBuf::from("src")])
            .expect_err("directory explicit file fails");
        assert!(error
            .to_string()
            .contains("explicit file is a directory: src"));
        fs::remove_dir_all(root).expect("cleanup");
    }
}
