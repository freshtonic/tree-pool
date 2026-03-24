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

/// Create a worktree with a new branch.
pub fn worktree_add_new_branch(repo_root: &Path, worktree_path: &Path, branch: &str) -> Result<()> {
    let path_str = worktree_path.to_str().context("invalid worktree path")?;
    run_git(repo_root, &["worktree", "add", "-b", branch, path_str])?;
    Ok(())
}

/// Create a worktree checking out an existing branch.
pub fn worktree_add_existing_branch(
    repo_root: &Path,
    worktree_path: &Path,
    branch: &str,
) -> Result<()> {
    let path_str = worktree_path.to_str().context("invalid worktree path")?;
    run_git(repo_root, &["worktree", "add", path_str, branch])?;
    Ok(())
}

/// Create a new branch and check it out.
pub fn create_and_checkout_branch(worktree_path: &Path, branch: &str) -> Result<()> {
    run_git(worktree_path, &["checkout", "-b", branch])?;
    Ok(())
}

/// Check out an existing branch.
pub fn checkout_branch(worktree_path: &Path, branch: &str) -> Result<()> {
    run_git(worktree_path, &["checkout", branch])?;
    Ok(())
}

/// Check if a branch exists locally or on a remote.
pub fn branch_exists(repo_root: &Path, branch: &str) -> Result<bool> {
    let local = format!("refs/heads/{branch}");
    let remote = format!("refs/remotes/origin/{branch}");
    let local_exists = run_git_ok(repo_root, &["rev-parse", "--verify", &local])?;
    let remote_exists = run_git_ok(repo_root, &["rev-parse", "--verify", &remote])?;
    Ok(local_exists || remote_exists)
}

/// List branch names sorted by most recent commit date (newest first).
/// Deduplicates local and remote branches (prefers local name).
pub fn list_branches_by_date(repo_root: &Path) -> Result<Vec<String>> {
    let output = run_git(
        repo_root,
        &[
            "branch",
            "--all",
            "--sort=-committerdate",
            "--format=%(refname:short)",
        ],
    )?;

    let mut seen = std::collections::HashSet::new();
    let mut branches = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.contains("HEAD") {
            continue;
        }
        let name = line.strip_prefix("origin/").unwrap_or(line);
        if seen.insert(name.to_string()) {
            branches.push(name.to_string());
        }
    }

    Ok(branches)
}

/// Return the set of branch names currently checked out in the main repo
/// and all its worktrees.
pub fn checked_out_branches(repo_root: &Path) -> Result<std::collections::HashSet<String>> {
    let output = run_git(repo_root, &["worktree", "list", "--porcelain"])?;
    let mut branches = std::collections::HashSet::new();

    for line in output.lines() {
        if let Some(branch) = line.strip_prefix("branch refs/heads/") {
            branches.insert(branch.to_string());
        }
    }

    Ok(branches)
}

/// Return the current branch name for a worktree, or None if detached.
pub fn current_branch(worktree_path: &Path) -> Result<Option<String>> {
    let output = run_git(worktree_path, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if output == "HEAD" {
        Ok(None)
    } else {
        Ok(Some(output))
    }
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

    #[test]
    fn test_list_branches_by_date() {
        let dir = setup_test_repo();
        let branches = list_branches_by_date(dir.path()).unwrap();
        assert!(!branches.is_empty());
    }

    #[test]
    fn test_checked_out_branches_includes_head() {
        let dir = setup_test_repo();
        let branches = checked_out_branches(dir.path()).unwrap();
        assert!(!branches.is_empty());
    }

    #[test]
    fn test_current_branch_detached() {
        let dir = setup_test_repo();
        Command::new("git")
            .args(["checkout", "--detach"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        let branch = current_branch(dir.path()).unwrap();
        assert!(branch.is_none());
    }

    #[test]
    fn test_current_branch_on_branch() {
        let dir = setup_test_repo();
        let branch = current_branch(dir.path()).unwrap();
        assert!(branch.is_some());
    }

    #[test]
    fn test_worktree_add_new_branch() {
        let dir = setup_test_repo();
        let wt_dir = tempfile::tempdir().unwrap();
        let wt_path = wt_dir.path().join("wt");
        worktree_add_new_branch(dir.path(), &wt_path, "feature/test").unwrap();
        assert!(wt_path.exists());
        let branch = current_branch(&wt_path).unwrap();
        assert_eq!(branch, Some("feature/test".to_string()));
    }

    #[test]
    fn test_worktree_add_existing_branch() {
        let dir = setup_test_repo();
        // Create a branch first
        Command::new("git")
            .args(["branch", "existing-branch"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        // Detach HEAD so the branch is free
        Command::new("git")
            .args(["checkout", "--detach"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let wt_dir = tempfile::tempdir().unwrap();
        let wt_path = wt_dir.path().join("wt");
        worktree_add_existing_branch(dir.path(), &wt_path, "existing-branch").unwrap();
        assert!(wt_path.exists());
        let branch = current_branch(&wt_path).unwrap();
        assert_eq!(branch, Some("existing-branch".to_string()));
    }

    #[test]
    fn test_branch_exists() {
        let dir = setup_test_repo();
        let branch = default_branch(dir.path()).unwrap();
        assert!(branch_exists(dir.path(), &branch).unwrap());
        assert!(!branch_exists(dir.path(), "nonexistent-branch-xyz").unwrap());
    }
}
