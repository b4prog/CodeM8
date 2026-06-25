use std::path::{Component, Path, PathBuf};

#[must_use]
pub fn format_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[must_use]
pub fn normalize_display_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => normalized.push(".."),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_paths_with_forward_slashes() {
        assert_eq!(
            format_path(Path::new("src\\nested\\a.ts")),
            "src/nested/a.ts"
        );
    }

    #[test]
    fn normalizes_display_paths_without_losing_parent_segments() {
        assert_eq!(
            normalize_display_path(Path::new("./src/../a.ts")),
            PathBuf::from("src").join("..").join("a.ts")
        );
    }

    #[test]
    fn normalizes_empty_display_path_to_current_directory() {
        assert_eq!(normalize_display_path(Path::new(".")), PathBuf::from("."));
    }
}
