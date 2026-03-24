use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::git;

/// Ensure the given path is listed in the nearest .gitignore.
/// No-op if the path is outside any git repo.
/// Idempotent — checks for existing entry before appending.
pub fn ensure_ignored(pool_dir: &Path) -> Result<()> {
    // Find the nearest git repo containing the pool dir.
    // Walk up from pool_dir to find an existing ancestor directory.
    let existing_ancestor = pool_dir
        .ancestors()
        .find(|p| p.exists() && p.is_dir())
        .unwrap_or(pool_dir);

    let repo_root = match git::repo_root(existing_ancestor) {
        Ok(root) => root,
        Err(_) => return Ok(()), // Not inside a git repo — no-op
    };

    let repo_root = Path::new(&repo_root);

    // Canonicalize both paths to handle symlinks (e.g. /tmp -> /private/tmp on macOS)
    let canonical_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());
    let canonical_pool = if pool_dir.exists() {
        pool_dir
            .canonicalize()
            .unwrap_or_else(|_| pool_dir.to_path_buf())
    } else {
        // Pool dir may not exist yet. Canonicalize the nearest existing ancestor
        // and re-append the remaining components.
        let mut existing = pool_dir.to_path_buf();
        let mut suffix_parts = Vec::new();
        while !existing.exists() {
            if let Some(file_name) = existing.file_name() {
                suffix_parts.push(file_name.to_os_string());
            }
            if !existing.pop() {
                break;
            }
        }
        let mut canonical = existing.canonicalize().unwrap_or_else(|_| existing.clone());
        for part in suffix_parts.into_iter().rev() {
            canonical.push(part);
        }
        canonical
    };

    // Compute the relative path from repo root to pool dir
    let rel_path = match canonical_pool.strip_prefix(&canonical_root) {
        Ok(rel) => rel,
        Err(_) => return Ok(()), // Pool dir is outside the repo — no-op
    };

    // Format as gitignore entry with forward slashes and leading /
    let entry = format!("/{}", rel_path.to_string_lossy().replace('\\', "/"));

    let gitignore_path = repo_root.join(".gitignore");

    // Check if entry already exists
    if gitignore_path.exists() {
        let contents = fs::read_to_string(&gitignore_path)
            .with_context(|| format!("failed to read {}", gitignore_path.display()))?;
        if contents.lines().any(|line| line.trim() == entry) {
            return Ok(());
        }
    }

    // Append the entry
    let mut contents = if gitignore_path.exists() {
        let mut c = fs::read_to_string(&gitignore_path)
            .with_context(|| format!("failed to read {}", gitignore_path.display()))?;
        if !c.ends_with('\n') && !c.is_empty() {
            c.push('\n');
        }
        c
    } else {
        String::new()
    };

    contents.push_str(&entry);
    contents.push('\n');

    fs::write(&gitignore_path, contents)
        .with_context(|| format!("failed to write {}", gitignore_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as Cmd;

    fn setup_test_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        Cmd::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .unwrap();
        Cmd::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(path)
            .output()
            .unwrap();
        Cmd::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(path)
            .output()
            .unwrap();
        std::fs::write(path.join("file.txt"), "hello").unwrap();
        Cmd::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        Cmd::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(path)
            .output()
            .unwrap();
        dir
    }

    #[test]
    fn adds_entry_to_gitignore() {
        let dir = setup_test_repo();
        let pool_dir = dir.path().join(".tree-pool").join("repo-abc123");
        ensure_ignored(&pool_dir).unwrap();
        let contents = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(contents.contains("/.tree-pool/repo-abc123"));
    }

    #[test]
    fn idempotent_does_not_duplicate() {
        let dir = setup_test_repo();
        let pool_dir = dir.path().join(".tree-pool").join("repo-abc123");
        ensure_ignored(&pool_dir).unwrap();
        ensure_ignored(&pool_dir).unwrap();
        let contents = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        let count = contents.matches("/.tree-pool/repo-abc123").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn preserves_existing_gitignore_content() {
        let dir = setup_test_repo();
        fs::write(dir.path().join(".gitignore"), "*.log\n").unwrap();
        let pool_dir = dir.path().join(".tree-pool").join("repo-abc123");
        ensure_ignored(&pool_dir).unwrap();
        let contents = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(contents.contains("*.log"));
        assert!(contents.contains("/.tree-pool/repo-abc123"));
    }
}
