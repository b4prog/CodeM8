use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use ignore::{DirEntry, WalkBuilder, WalkState};

use crate::error::{CodeM8Error, Result};
use crate::model::SourceFile;
use crate::paths::{format_path, normalize_display_path};

const IGNORED_DIRECTORIES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "coverage",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".idea",
    ".vscode",
];

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
        discover_explicit_files(current_dir, extensions, files)?
    } else {
        discover_recursive_files(current_dir, extensions)?
    };
    source_files.sort_by(|left, right| {
        format_path(&left.display_path).cmp(&format_path(&right.display_path))
    });
    Ok(source_files)
}

fn discover_recursive_files(root: &Path, extensions: &[String]) -> Result<Vec<SourceFile>> {
    let root = root.to_path_buf();
    let extensions = extensions.to_vec();
    let (source_tx, source_rx) = mpsc::channel();
    let (error_tx, error_rx) = mpsc::channel();
    let walker = WalkBuilder::new(&root)
        .hidden(false)
        .ignore(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false)
        .parents(true)
        .filter_entry(should_walk_entry)
        .build_parallel();
    walker.run(|| {
        let root = root.clone();
        let extensions = extensions.clone();
        let source_tx = source_tx.clone();
        let error_tx = error_tx.clone();
        Box::new(move |entry| match entry {
            Ok(entry) => {
                let Some(source_file) = source_file_from_entry(&root, &extensions, &entry) else {
                    return WalkState::Continue;
                };
                if source_tx.send(source_file).is_err() {
                    return WalkState::Quit;
                }
                WalkState::Continue
            }
            Err(error) => {
                let _ = error_tx.send(walk_error(&root, &error));
                WalkState::Quit
            }
        })
    });
    drop(source_tx);
    drop(error_tx);
    if let Some(error) = error_rx.into_iter().next() {
        return Err(error);
    }
    Ok(source_rx.into_iter().collect())
}

fn source_file_from_entry(
    root: &Path,
    extensions: &[String],
    entry: &DirEntry,
) -> Option<SourceFile> {
    let file_type = entry.file_type()?;
    if !file_type.is_file() {
        return None;
    }
    let path = entry.path();
    let extension = selected_extension(path, extensions)?;
    let display_path = path
        .strip_prefix(root)
        .map_or_else(|_| normalize_display_path(path), normalize_display_path);
    Some(SourceFile {
        path: path.to_path_buf(),
        display_path,
        extension,
    })
}

fn walk_error(root: &Path, error: &ignore::Error) -> CodeM8Error {
    CodeM8Error::new(format!(
        "could not walk directory {}: {error}",
        format_path(root)
    ))
}

fn should_walk_entry(entry: &DirEntry) -> bool {
    let Some(file_type) = entry.file_type() else {
        return true;
    };
    if !file_type.is_dir() || entry.depth() == 0 {
        return true;
    }
    let directory_name = entry.file_name().to_string_lossy().to_ascii_lowercase();
    !IGNORED_DIRECTORIES.contains(&directory_name.as_str())
}

fn discover_explicit_files(
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

fn selected_extension(path: &Path, extensions: &[String]) -> Option<String> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    extensions
        .iter()
        .any(|selected| selected.eq_ignore_ascii_case(&extension))
        .then_some(extension)
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
            "codem8-discovery-{name}-{}-{id}",
            std::process::id()
        ));
        if path.exists() {
            fs::remove_dir_all(&path).expect("remove stale test directory");
        }
        fs::create_dir_all(&path).expect("create test directory");
        path
    }

    #[test]
    fn recursively_discovers_matching_extensions_and_ignores_common_directories() {
        let root = temp_dir("recursive");
        fs::create_dir_all(root.join("src")).expect("create src");
        fs::create_dir_all(root.join("target")).expect("create target");
        fs::write(root.join("src").join("a.TS"), "").expect("write ts");
        fs::write(root.join("src").join("b.js"), "").expect("write js");
        fs::write(root.join("target").join("ignored.ts"), "").expect("write ignored");
        let files = discover_source_files(&root, &["ts".to_string()], None).expect("discover");
        assert_eq!(files.len(), 1);
        assert_eq!(format_path(&files[0].display_path), "src/a.TS");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn recursive_discovery_respects_gitignore_without_requiring_git_repository() {
        let root = temp_dir("gitignore");
        fs::create_dir_all(root.join("src")).expect("create src");
        fs::create_dir_all(root.join("generated")).expect("create generated");
        fs::write(root.join(".gitignore"), "generated/\n").expect("write gitignore");
        fs::write(root.join("src").join("a.ts"), "").expect("write source ts");
        fs::write(root.join("generated").join("ignored.ts"), "").expect("write ignored ts");
        let files = discover_source_files(&root, &["ts".to_string()], None).expect("discover");
        assert_eq!(files.len(), 1);
        assert_eq!(format_path(&files[0].display_path), "src/a.ts");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn explicit_files_skip_unselected_extensions() {
        let root = temp_dir("explicit-skip");
        fs::write(root.join("a.ts"), "").expect("write ts");
        fs::write(root.join("b.js"), "").expect("write js");
        let files = discover_source_files(
            &root,
            &["ts".to_string()],
            Some(&[PathBuf::from("a.ts"), PathBuf::from("b.js")]),
        )
        .expect("discover");
        assert_eq!(files.len(), 1);
        assert_eq!(format_path(&files[0].display_path), "a.ts");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn explicit_files_deduplicate_resolved_paths() {
        let root = temp_dir("explicit-dedup");
        fs::write(root.join("a.ts"), "").expect("write ts");
        let absolute = fs::canonicalize(root.join("a.ts")).expect("canonicalize ts");
        let files = discover_source_files(
            &root,
            &["ts".to_string()],
            Some(&[
                PathBuf::from("a.ts"),
                PathBuf::from(".").join("a.ts"),
                absolute.clone(),
            ]),
        )
        .expect("discover");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, absolute);
        assert_eq!(format_path(&files[0].display_path), "a.ts");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn explicit_files_reject_directories() {
        let root = temp_dir("explicit-directory");
        fs::create_dir_all(root.join("src")).expect("create explicit directory");
        let error =
            discover_source_files(&root, &["ts".to_string()], Some(&[PathBuf::from("src")]))
                .expect_err("directory explicit file fails");
        assert!(error
            .to_string()
            .contains("explicit file is a directory: src"));
        fs::remove_dir_all(root).expect("cleanup");
    }
}
