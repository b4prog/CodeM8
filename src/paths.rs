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
