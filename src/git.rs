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

pub fn is_dirty(repo: &Path) -> Result<bool> {
    let output = run_git(repo, &["status", "--porcelain"])?;
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
    if !has_remote(repo_root, "origin")? {
        return Ok(None);
    }
    let url = run_git(repo_root, &["remote", "get-url", "origin"])?;
    Ok(Some(url))
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

pub fn reset_tree(tree_path: &Path, ref_name: &str) -> Result<()> {
    run_git(tree_path, &["checkout", "--detach", ref_name])?;
    run_git(tree_path, &["reset", "--hard"])?;
    Ok(())
}

/// Create a new branch and check it out.
pub fn create_and_checkout_branch(repo: &Path, branch: &str) -> Result<()> {
    run_git(repo, &["checkout", "-b", branch])?;
    Ok(())
}

/// Check out an existing branch.
pub fn checkout_branch(repo: &Path, branch: &str) -> Result<()> {
    run_git(repo, &["checkout", branch])?;
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

/// Return the current branch name for a tree, or None if detached.
pub fn current_branch(repo: &Path) -> Result<Option<String>> {
    let output = run_git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if output == "HEAD" {
        Ok(None)
    } else {
        Ok(Some(output))
    }
}

/// Clone a repository using --local for hardlinked objects.
pub fn clone_local(source: &Path, dest: &Path) -> Result<()> {
    let source_str = source.to_str().context("invalid source path")?;
    let dest_str = dest.to_str().context("invalid dest path")?;
    // Run from source's parent since clone creates the dest directory
    let parent = source.parent().unwrap_or(source);
    run_git(parent, &["clone", "--local", source_str, dest_str])?;
    Ok(())
}

/// Rename a remote.
pub fn rename_remote(repo: &Path, old: &str, new: &str) -> Result<()> {
    run_git(repo, &["remote", "rename", old, new])?;
    Ok(())
}

/// Add a new remote.
pub fn add_remote(repo: &Path, name: &str, url: &str) -> Result<()> {
    run_git(repo, &["remote", "add", name, url])?;
    Ok(())
}

/// Check if a remote exists by name.
pub fn has_remote(repo: &Path, name: &str) -> Result<bool> {
    let remotes = run_git(repo, &["remote"])?;
    Ok(remotes.lines().any(|r| r == name))
}

/// Fetch from a specific remote. Returns Ok(()) on success, Err on failure.
pub fn fetch_remote(repo: &Path, remote: &str) -> Result<()> {
    run_git(repo, &["fetch", remote])?;
    Ok(())
}

/// Return the list of local branch names that have no corresponding branch
/// on any remote, or are ahead of all remotes.
pub fn unpushed_branches(repo: &Path) -> Result<Vec<String>> {
    let output = run_git(repo, &["branch", "--format=%(refname:short)"])?;
    let remotes_output = run_git(repo, &["remote"])?;
    let mut unpushed = Vec::new();

    for branch in output.lines() {
        let branch = branch.trim();
        if branch.is_empty() {
            continue;
        }

        // Check if any remote has this branch at the same or newer commit
        let mut pushed = false;

        for remote in remotes_output.lines() {
            let remote = remote.trim();
            if remote.is_empty() {
                continue;
            }
            let remote_ref = format!("{remote}/{branch}");
            // Check if remote ref exists
            if !run_git_ok(repo, &["rev-parse", "--verify", &remote_ref])? {
                continue;
            }
            // Check if local is ancestor-or-equal to remote (i.e., not ahead)
            let local_is_ancestor =
                run_git_ok(repo, &["merge-base", "--is-ancestor", branch, &remote_ref])?;
            if local_is_ancestor {
                pushed = true;
                break;
            }
        }

        if !pushed {
            unpushed.push(branch.to_string());
        }
    }

    Ok(unpushed)
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
    fn test_branch_exists() {
        let dir = setup_test_repo();
        let branch = default_branch(dir.path()).unwrap();
        assert!(branch_exists(dir.path(), &branch).unwrap());
        assert!(!branch_exists(dir.path(), "nonexistent-branch-xyz").unwrap());
    }

    #[test]
    fn test_clone_local() {
        let source = setup_test_repo();
        let dest_dir = tempfile::tempdir().unwrap();
        let dest = dest_dir.path().join("clone");
        clone_local(source.path(), &dest).unwrap();
        assert!(dest.join(".git").exists());
        // Clone should have the source as origin
        let url = remote_url(&dest).unwrap().unwrap();
        assert!(url.contains(source.path().to_str().unwrap()));
    }

    #[test]
    fn test_rename_remote() {
        let source = setup_test_repo();
        let dest_dir = tempfile::tempdir().unwrap();
        let dest = dest_dir.path().join("clone");
        clone_local(source.path(), &dest).unwrap();
        rename_remote(&dest, "origin", "local").unwrap();
        // "origin" should no longer exist
        assert!(!has_remote(&dest, "origin").unwrap());
        // "local" should exist
        assert!(has_remote(&dest, "local").unwrap());
    }

    #[test]
    fn test_add_remote() {
        let dir = setup_test_repo();
        add_remote(dir.path(), "upstream", "https://example.com/repo.git").unwrap();
        assert!(has_remote(dir.path(), "upstream").unwrap());
    }

    #[test]
    fn test_fetch_remote_existing() {
        let source = setup_test_repo();
        let dest_dir = tempfile::tempdir().unwrap();
        let dest = dest_dir.path().join("clone");
        clone_local(source.path(), &dest).unwrap();
        // Fetching origin (which points to local source) should succeed
        fetch_remote(&dest, "origin").unwrap();
    }

    #[test]
    fn test_has_remote() {
        let dir = setup_test_repo();
        assert!(!has_remote(dir.path(), "origin").unwrap());
        add_remote(dir.path(), "origin", "https://example.com/repo.git").unwrap();
        assert!(has_remote(dir.path(), "origin").unwrap());
    }

    #[test]
    fn test_unpushed_branches_none_when_pushed() {
        let source = setup_test_repo();
        let dest_dir = tempfile::tempdir().unwrap();
        let dest = dest_dir.path().join("clone");
        clone_local(source.path(), &dest).unwrap();
        // Default branch should be tracked by origin — no unpushed branches
        let unpushed = unpushed_branches(&dest).unwrap();
        assert!(unpushed.is_empty());
    }

    #[test]
    fn test_reset_tree() {
        let dir = setup_test_repo();
        // Create a new branch with a commit
        run_git(dir.path(), &["checkout", "-b", "feature/dirty"]).unwrap();
        std::fs::write(dir.path().join("extra.txt"), "dirty").unwrap();
        run_git(dir.path(), &["add", "."]).unwrap();
        run_git(dir.path(), &["commit", "-m", "dirty commit"]).unwrap();
        // Also leave an untracked file (simulates build artifacts, etc.)
        std::fs::write(dir.path().join("untracked.txt"), "junk").unwrap();

        // Reset to default branch ref
        let default = default_branch(dir.path()).unwrap();
        let ref_name = format!("refs/heads/{default}");
        reset_tree(dir.path(), &ref_name).unwrap();

        // Should be detached HEAD
        let branch = current_branch(dir.path()).unwrap();
        assert!(branch.is_none(), "expected detached HEAD after reset_tree");

        // Untracked files should be preserved (build artifacts like target/)
        assert!(
            dir.path().join("untracked.txt").exists(),
            "untracked files should be preserved by reset_tree"
        );
    }

    #[test]
    fn test_unpushed_branches_detects_new_branch() {
        let source = setup_test_repo();
        let dest_dir = tempfile::tempdir().unwrap();
        let dest = dest_dir.path().join("clone");
        clone_local(source.path(), &dest).unwrap();
        run_git(&dest, &["config", "user.email", "test@test.com"]).unwrap();
        run_git(&dest, &["config", "user.name", "Test"]).unwrap();
        // Create a new branch with a commit
        run_git(&dest, &["checkout", "-b", "feature/new"]).unwrap();
        std::fs::write(dest.join("new.txt"), "content").unwrap();
        run_git(&dest, &["add", "."]).unwrap();
        run_git(&dest, &["commit", "-m", "new commit"]).unwrap();
        let unpushed = unpushed_branches(&dest).unwrap();
        assert!(unpushed.contains(&"feature/new".to_string()));
    }
}
