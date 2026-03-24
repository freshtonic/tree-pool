use std::path::Path;

/// Replace the home directory prefix with ~ for display.
pub fn pretty_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(rel) = path.strip_prefix(&home) {
            return format!("~/{}", rel.display());
        }
    }
    path.display().to_string()
}

/// Check if the given path matches the current working directory.
pub fn is_current_dir(path: &Path) -> bool {
    let Ok(cwd) = std::env::current_dir() else {
        return false;
    };
    // Canonicalize both to handle symlinks
    let cwd = cwd.canonicalize().unwrap_or(cwd);
    let path = path.canonicalize().unwrap_or(path.to_path_buf());
    cwd == path || cwd.starts_with(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_path_replaces_home() {
        if let Some(home) = dirs::home_dir() {
            let path = home.join("projects").join("test");
            let pretty = pretty_path(&path);
            assert!(pretty.starts_with("~/"));
            assert!(pretty.contains("projects/test"));
        }
    }

    #[test]
    fn pretty_path_leaves_non_home_alone() {
        let path = Path::new("/tmp/something");
        let pretty = pretty_path(path);
        assert_eq!(pretty, "/tmp/something");
    }
}
