const CARGO_LOCK: &str = include_str!("../../Cargo.lock");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CargoLockPackage<'a> {
    name: &'a str,
    version: &'a str,
}

pub(super) fn codem8_version_from_cargo_lock() -> Option<&'static str> {
    cargo_lock_packages(CARGO_LOCK)
        .find(|package| package.name == "codem8")
        .map(|package| package.version)
}

fn cargo_lock_packages(lockfile: &str) -> impl Iterator<Item = CargoLockPackage<'_>> {
    lockfile.split("[[package]]").filter_map(cargo_lock_package)
}

fn cargo_lock_package(section: &str) -> Option<CargoLockPackage<'_>> {
    let name = cargo_lock_value(section, "name")?;
    let version = cargo_lock_value(section, "version")?;
    Some(CargoLockPackage { name, version })
}

fn cargo_lock_value<'a>(section: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key} = \"");
    section
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(&prefix)?.strip_suffix('"'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_package_versions_from_cargo_lock_sections() {
        let lockfile = r#"
[[package]]
name = "dependency"
version = "1.2.3"

[[package]]
name = "codem8"
version = "0.4.2"
"#;
        let package = cargo_lock_packages(lockfile)
            .find(|package| package.name == "codem8")
            .expect("package exists");
        assert_eq!(package.version, "0.4.2");
    }
}
