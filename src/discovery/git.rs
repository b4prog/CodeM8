use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::error::{CodeM8Error, Result};

/// Lists files changed on the current branch compared to the origin base branch.
///
/// # Errors
///
/// Returns an error when `current_dir` is not inside a Git repository, the
/// current branch cannot be resolved, or the origin base branch is missing.
pub fn changed_files_against_origin(current_dir: &Path) -> Result<Vec<PathBuf>> {
    let repo_root = repo_root(current_dir)?;
    ensure_named_branch(&repo_root)?;
    let origin_ref = origin_base_ref(&repo_root)?;
    let merge_base = run_git_text(
        &repo_root,
        &["merge-base", &origin_ref, "HEAD"],
        "find merge base with origin base branch",
    )?;
    let mut paths = BTreeSet::new();
    collect_nul_paths(
        &repo_root,
        &[
            "diff",
            "--name-only",
            "-z",
            "--diff-filter=ACMRTUXB",
            merge_base.trim(),
            "HEAD",
        ],
        &mut paths,
    )?;
    collect_nul_paths(
        &repo_root,
        &[
            "diff",
            "--name-only",
            "-z",
            "--cached",
            "--diff-filter=ACMRTUXB",
        ],
        &mut paths,
    )?;
    collect_nul_paths(
        &repo_root,
        &["diff", "--name-only", "-z", "--diff-filter=ACMRTUXB"],
        &mut paths,
    )?;
    collect_nul_paths(
        &repo_root,
        &["ls-files", "--others", "--exclude-standard", "-z"],
        &mut paths,
    )?;
    Ok(paths
        .into_iter()
        .filter_map(|path| existing_file_path(&repo_root, current_dir, &path))
        .collect())
}

fn repo_root(current_dir: &Path) -> Result<PathBuf> {
    let output = run_git_output(
        current_dir,
        &["rev-parse", "--show-toplevel"],
        "find git repository",
    )?;
    if !output.status.success() {
        return Err(CodeM8Error::new(
            "git branch mode requires the current directory to be inside a git repository",
        ));
    }
    let root = output_text(output.stdout, "parse git repository root")?;
    Ok(PathBuf::from(root.trim()))
}

fn ensure_named_branch(repo_root: &Path) -> Result<()> {
    let branch = run_git_text(
        repo_root,
        &["rev-parse", "--abbrev-ref", "HEAD"],
        "determine current git branch",
    )?;
    let branch = branch.trim();
    if branch == "HEAD" {
        return Err(CodeM8Error::new(
            "git branch mode requires a named local branch, but HEAD is detached",
        ));
    }
    Ok(())
}

fn origin_base_ref(repo_root: &Path) -> Result<String> {
    for candidate in ["origin/HEAD", "origin/main", "origin/master"] {
        if verify_origin_ref(repo_root, candidate) {
            return Ok(candidate.to_string());
        }
    }
    Err(CodeM8Error::new(
        "git branch mode could not resolve origin base branch",
    ))
}

fn verify_origin_ref(repo_root: &Path, origin_ref: &str) -> bool {
    let commit_ref = format!("{origin_ref}^{{commit}}");
    run_git_output(
        repo_root,
        &["rev-parse", "--verify", &commit_ref],
        "resolve origin base branch",
    )
    .is_ok_and(|output| output.status.success())
}

fn collect_nul_paths(repo_root: &Path, args: &[&str], paths: &mut BTreeSet<PathBuf>) -> Result<()> {
    let output = run_git_output(repo_root, args, "list changed git files")?;
    let stdout = ensure_git_success(output, "list changed git files")?;
    for path in nul_paths(&stdout) {
        paths.insert(path);
    }
    Ok(())
}

fn existing_file_path(repo_root: &Path, current_dir: &Path, path: &Path) -> Option<PathBuf> {
    let absolute = repo_root.join(path);
    let metadata = fs::symlink_metadata(&absolute).ok()?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return None;
    }
    let relative = absolute.strip_prefix(current_dir).map(Path::to_path_buf);
    Some(relative.unwrap_or(absolute))
}

fn run_git_text(current_dir: &Path, args: &[&str], action: &str) -> Result<String> {
    let output = run_git_output(current_dir, args, action)?;
    let stdout = ensure_git_success(output, action)?;
    output_text(stdout, action)
}

fn run_git_output(current_dir: &Path, args: &[&str], action: &str) -> Result<Output> {
    Command::new("git")
        .arg("-C")
        .arg(current_dir)
        .args(args)
        .output()
        .map_err(|error| CodeM8Error::new(format!("could not {action}: {error}")))
}

fn ensure_git_success(output: Output, action: &str) -> Result<Vec<u8>> {
    if output.status.success() {
        return Ok(output.stdout);
    }
    let stderr = output_text(output.stderr, action)?;
    Err(CodeM8Error::new(format!(
        "could not {action}: {}",
        stderr.trim()
    )))
}

fn output_text(bytes: Vec<u8>, action: &str) -> Result<String> {
    String::from_utf8(bytes)
        .map_err(|error| CodeM8Error::new(format!("could not {action}: {error}")))
}

fn nul_paths(bytes: &[u8]) -> Vec<PathBuf> {
    String::from_utf8_lossy(bytes)
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempGitRepo {
        path: PathBuf,
    }

    impl TempGitRepo {
        fn new(name: &str) -> Self {
            let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("codem8-git-{name}-{}-{id}", std::process::id()));
            if path.exists() {
                fs::remove_dir_all(&path).expect("remove stale test directory");
            }
            fs::create_dir_all(&path).expect("create test directory");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn write(&self, relative_path: &str, contents: &str) {
            let path = self.path.join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create parent directory");
            }
            fs::write(path, contents).expect("write test file");
        }

        fn git(&self, args: &[&str]) {
            let status = Command::new("git")
                .arg("-C")
                .arg(&self.path)
                .args(args)
                .status()
                .expect("run git");
            assert!(status.success(), "git command failed: {args:?}");
        }

        fn commit(&self, message: &str) {
            self.git(&["add", "."]);
            self.git(&[
                "-c",
                "user.name=CodeM8 Test",
                "-c",
                "user.email=codem8@example.invalid",
                "commit",
                "-m",
                message,
            ]);
        }
    }

    impl Drop for TempGitRepo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn git_is_available() -> bool {
        Command::new("git")
            .arg("--version")
            .status()
            .is_ok_and(|status| status.success())
    }

    #[test]
    fn rejects_non_git_directory() {
        let repo = TempGitRepo::new("non-repo");
        let error = changed_files_against_origin(repo.path()).expect_err("non-repo fails");
        assert!(error.to_string().contains("requires the current directory"));
    }

    #[test]
    fn lists_committed_staged_unstaged_and_untracked_files() {
        if !git_is_available() {
            return;
        }
        let repo = TempGitRepo::new("changes");
        repo.git(&["init"]);
        repo.write("src/base.ts", "const value = one;\n");
        repo.write("src/deleted.ts", "const value = deleted;\n");
        repo.commit("initial");
        repo.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        repo.git(&["branch", "-M", "feature"]);
        repo.write("src/committed.ts", "const value = committed;\n");
        repo.commit("branch change");
        repo.git(&["update-ref", "refs/remotes/origin/feature", "HEAD"]);
        repo.write("src/staged.ts", "const value = staged;\n");
        repo.git(&["add", "src/staged.ts"]);
        repo.write("src/base.ts", "const value = modified;\n");
        repo.write("src/untracked.ts", "const value = untracked;\n");
        fs::remove_file(repo.path().join("src/deleted.ts")).expect("delete tracked file");
        let files = changed_files_against_origin(repo.path()).expect("list branch files");
        assert_eq!(
            files,
            [
                PathBuf::from("src/base.ts"),
                PathBuf::from("src/committed.ts"),
                PathBuf::from("src/staged.ts"),
                PathBuf::from("src/untracked.ts"),
            ]
        );
    }
}
