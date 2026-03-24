use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Run a git command in the given directory and return stdout as a trimmed string.
/// Returns an error if git exits non-zero, including stderr in the message.
fn run_git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .context("failed to run git")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {}: {}", args.join(" "), stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(stdout)
}

/// Run a git command and return whether it succeeded (exit 0).
fn run_git_ok(dir: &Path, args: &[&str]) -> Result<bool> {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("failed to run git")?;

    Ok(status.success())
}

pub fn repo_root(dir: &Path) -> Result<String> {
    run_git(dir, &["rev-parse", "--show-toplevel"])
}

pub fn is_dirty(worktree_path: &Path) -> Result<bool> {
    let output = run_git(worktree_path, &["status", "--porcelain"])?;
    Ok(!output.is_empty())
}

pub fn default_branch(repo_root: &Path) -> Result<String> {
    // Try: git symbolic-ref refs/remotes/origin/HEAD
    if let Ok(refname) = run_git(repo_root, &["symbolic-ref", "refs/remotes/origin/HEAD"])
        && let Some(branch) = refname.strip_prefix("refs/remotes/origin/")
    {
        return Ok(branch.to_string());
    }

    // Try: git symbolic-ref HEAD
    if let Ok(refname) = run_git(repo_root, &["symbolic-ref", "HEAD"])
        && let Some(branch) = refname.strip_prefix("refs/heads/")
    {
        return Ok(branch.to_string());
    }

    // Try: git config init.defaultBranch
    if let Ok(branch) = run_git(repo_root, &["config", "init.defaultBranch"])
        && !branch.is_empty()
    {
        return Ok(branch);
    }

    bail!("could not determine default branch — try running: git remote set-head origin --auto")
}

pub fn remote_url(repo_root: &Path) -> Result<Option<String>> {
    if !has_origin(repo_root)? {
        return Ok(None);
    }
    let url = run_git(repo_root, &["remote", "get-url", "origin"])?;
    Ok(Some(url))
}

pub fn has_origin(repo_root: &Path) -> Result<bool> {
    let remotes = run_git(repo_root, &["remote"])?;
    Ok(remotes.lines().any(|r| r == "origin"))
}

pub fn fetch_origin(repo_root: &Path) -> Result<()> {
    run_git(repo_root, &["fetch", "origin"])?;
    Ok(())
}

/// Determine the best ref for the given branch, comparing local vs remote.
/// Prefers the one that is further ahead. On divergence, prefers origin.
pub fn branch_ref(repo_root: &Path, branch: &str) -> Result<String> {
    let local_ref = format!("refs/heads/{branch}");
    let remote_ref = format!("origin/{branch}");

    let local_exists = run_git_ok(repo_root, &["rev-parse", "--verify", &local_ref])?;
    let remote_exists = run_git_ok(repo_root, &["rev-parse", "--verify", &remote_ref])?;

    match (local_exists, remote_exists) {
        (false, false) => bail!("neither local nor remote ref found for branch {branch}"),
        (true, false) => Ok(local_ref),
        (false, true) => Ok(remote_ref),
        (true, true) => {
            // Check if local is ancestor of remote
            let local_is_ancestor = run_git_ok(
                repo_root,
                &["merge-base", "--is-ancestor", &local_ref, &remote_ref],
            )?;
            if local_is_ancestor {
                return Ok(remote_ref);
            }

            // Check if remote is ancestor of local
            let remote_is_ancestor = run_git_ok(
                repo_root,
                &["merge-base", "--is-ancestor", &remote_ref, &local_ref],
            )?;
            if remote_is_ancestor {
                return Ok(local_ref);
            }

            // Diverged — prefer remote
            Ok(remote_ref)
        }
    }
}

pub fn worktree_add(repo_root: &Path, worktree_path: &Path, ref_name: &str) -> Result<()> {
    let path_str = worktree_path.to_str().context("invalid worktree path")?;
    run_git(
        repo_root,
        &["worktree", "add", "--detach", path_str, ref_name],
    )?;
    Ok(())
}

pub fn worktree_remove(repo_root: &Path, worktree_path: &Path) -> Result<()> {
    let path_str = worktree_path.to_str().context("invalid worktree path")?;
    run_git(repo_root, &["worktree", "remove", "--force", path_str])?;
    Ok(())
}

pub fn reset_worktree(worktree_path: &Path, ref_name: &str) -> Result<()> {
    run_git(worktree_path, &["checkout", "--detach", ref_name])?;
    run_git(worktree_path, &["reset", "--hard"])?;
    run_git(worktree_path, &["clean", "-fd"])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// Create a temporary git repo for testing and return its path.
    fn setup_test_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(path)
            .output()
            .unwrap();
        // Create initial commit so HEAD exists
        std::fs::write(path.join("file.txt"), "hello").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(path)
            .output()
            .unwrap();
        dir
    }

    #[test]
    fn test_repo_root() {
        let dir = setup_test_repo();
        let root = repo_root(dir.path()).unwrap();
        // Canonicalize both to handle symlinks like /private/tmp on macOS
        let expected = dir.path().canonicalize().unwrap();
        let actual = Path::new(&root).canonicalize().unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_is_dirty_clean_repo() {
        let dir = setup_test_repo();
        assert!(!is_dirty(dir.path()).unwrap());
    }

    #[test]
    fn test_is_dirty_with_changes() {
        let dir = setup_test_repo();
        std::fs::write(dir.path().join("new.txt"), "dirty").unwrap();
        assert!(is_dirty(dir.path()).unwrap());
    }

    #[test]
    fn test_default_branch_from_head() {
        let dir = setup_test_repo();
        let branch = default_branch(dir.path()).unwrap();
        // Should return whatever branch was created by init (main or master)
        assert!(!branch.is_empty());
    }

    #[test]
    fn test_remote_url_no_remote() {
        let dir = setup_test_repo();
        let url = remote_url(dir.path()).unwrap();
        assert!(url.is_none());
    }
}
