use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::git;

/// Resolve the pool directory path for a given repo.
/// Format: <root>/.tree-pool/<repoName>-<shortHash>/
pub fn resolve_pool_dir(repo_root: &Path, config: &Config) -> Result<PathBuf> {
    let base = config.resolve_root(repo_root)?;
    let repo_name = repo_root
        .file_name()
        .context("repo root has no name")?
        .to_string_lossy();

    let hash_input = match git::remote_url(repo_root)? {
        Some(url) => url,
        None => repo_root
            .canonicalize()
            .unwrap_or_else(|_| repo_root.to_path_buf())
            .to_string_lossy()
            .to_string(),
    };

    let short_hash = short_sha256(&hash_input);
    let dir_name = format!("{repo_name}-{short_hash}");

    Ok(base.join(".tree-pool").join(dir_name))
}

/// Compute the first 6 hex chars of SHA-256.
fn short_sha256(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    // First 3 bytes = 6 hex chars
    hex::encode(&result[..3])
}

/// Compute the tree path within the pool.
/// Format: <poolDir>/<name>/<repoName>/
pub fn tree_path(pool_dir: &Path, name: &str, repo_name: &str) -> PathBuf {
    pool_dir.join(name).join(repo_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_sha256_consistent() {
        let hash = short_sha256("https://github.com/user/repo.git");
        assert_eq!(hash.len(), 6);
        // Same input always produces same output
        assert_eq!(hash, short_sha256("https://github.com/user/repo.git"));
    }

    #[test]
    fn short_sha256_different_inputs() {
        let h1 = short_sha256("input1");
        let h2 = short_sha256("input2");
        assert_ne!(h1, h2);
    }

    #[test]
    fn tree_path_format() {
        let pool_dir = Path::new("/home/user/.tree-pool/myrepo-abc123");
        let path = tree_path(pool_dir, "1", "myrepo");
        assert_eq!(
            path,
            PathBuf::from("/home/user/.tree-pool/myrepo-abc123/1/myrepo")
        );
    }
}
