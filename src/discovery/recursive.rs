use std::path::Path;
use std::sync::mpsc;

use ignore::{DirEntry, WalkBuilder, WalkState};

use super::selected_extension;
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

pub(super) fn discover_recursive_files(
    root: &Path,
    extensions: &[String],
) -> Result<Vec<SourceFile>> {
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir(name: &str) -> PathBuf {
        let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "codem8-discovery-recursive-{name}-{}-{id}",
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
        let files = discover_recursive_files(&root, &["ts".to_string()]).expect("discover");
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
        let files = discover_recursive_files(&root, &["ts".to_string()]).expect("discover");
        assert_eq!(files.len(), 1);
        assert_eq!(format_path(&files[0].display_path), "src/a.ts");
        fs::remove_dir_all(root).expect("cleanup");
    }
}
