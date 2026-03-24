# tree-pool Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use cipherpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a Rust CLI tool (`tp`) that manages a pool of reusable git worktrees for parallel AI coding agent workflows.

**Architecture:** Modular design with separate modules for git operations, config, state management, process detection, and pool management. All git operations shell out to the system `git` binary. State is persisted as JSON with file-locking for concurrent access safety.

**Tech Stack:** Rust 2024 edition, clap (derive), serde/serde_json/toml, colored, sysinfo, fs2, sha2

---

### Task 1: Project Setup and Dependencies

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/main.rs`

**Step 1: Update Cargo.toml with correct crate name and dependencies**

Replace the entire contents of `Cargo.toml`:

```toml
[package]
name = "tree-pool"
version = "0.1.0"
edition = "2024"
description = "A CLI tool for managing pools of reusable git worktrees"
license = "MIT"

[[bin]]
name = "tp"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
colored = "3"
sysinfo = "0.34"
fs2 = "0.4"
sha2 = "0.10"
chrono = { version = "0.4", features = ["serde"] }
dirs = "6"
anyhow = "1"
```

**Step 2: Write a minimal main.rs to verify the build**

Replace `src/main.rs`:

```rust
fn main() {
    println!("tp - tree-pool worktree manager");
}
```

**Step 3: Build to verify dependencies resolve**

Run: `cargo build`
Expected: Compiles successfully.

**Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs
git commit -m "chore: set up tree-pool project with dependencies"
```

---

### Task 2: CLI Skeleton with clap

**Files:**
- Modify: `src/main.rs`
- Create: `src/cli.rs`

**Step 1: Write a test for CLI parsing**

Add to the bottom of `src/cli.rs`:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tp", version, about = "Manage a pool of reusable git worktrees")]
#[command(disable_help_subcommand = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Acquire a worktree from the pool and open a subshell
    Get,
    /// Show pool status
    Status,
    /// Return a worktree to the pool
    Return {
        /// Path to the worktree to return
        path: Option<String>,
        /// Skip dirty-check prompt
        #[arg(long)]
        force: bool,
    },
    /// Remove a worktree from the pool permanently
    Destroy {
        /// Path to the worktree to destroy
        path: Option<String>,
        /// Force destroy even if in-use
        #[arg(long)]
        force: bool,
        /// Destroy all worktrees
        #[arg(long)]
        all: bool,
    },
    /// Create tree-pool.toml in the repo root
    Init,
    /// Update tree-pool via cargo install
    Update,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn no_subcommand_is_none() {
        let cli = Cli::parse_from(["tp"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn get_subcommand() {
        let cli = Cli::parse_from(["tp", "get"]);
        assert!(matches!(cli.command, Some(Command::Get)));
    }

    #[test]
    fn status_subcommand() {
        let cli = Cli::parse_from(["tp", "status"]);
        assert!(matches!(cli.command, Some(Command::Status)));
    }

    #[test]
    fn return_with_force() {
        let cli = Cli::parse_from(["tp", "return", "--force", "/some/path"]);
        match cli.command {
            Some(Command::Return { path, force }) => {
                assert_eq!(path.as_deref(), Some("/some/path"));
                assert!(force);
            }
            _ => panic!("expected Return command"),
        }
    }

    #[test]
    fn destroy_all() {
        let cli = Cli::parse_from(["tp", "destroy", "--all"]);
        match cli.command {
            Some(Command::Destroy { all, .. }) => assert!(all),
            _ => panic!("expected Destroy command"),
        }
    }

    #[test]
    fn version_flag() {
        let result = Cli::try_parse_from(["tp", "--version"]);
        assert!(result.is_err()); // clap exits on --version
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }
}
```

**Step 2: Run tests to verify they pass**

Run: `cargo test --lib cli::tests`
Expected: All 6 tests pass.

**Step 3: Wire up main.rs**

Replace `src/main.rs`:

```rust
mod cli;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        None | Some(Command::Get) => cmd_get(),
        Some(Command::Status) => cmd_status(),
        Some(Command::Return { path, force }) => cmd_return(path, force),
        Some(Command::Destroy { path, force, all }) => cmd_destroy(path, force, all),
        Some(Command::Init) => cmd_init(),
        Some(Command::Update) => cmd_update(),
    };

    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn cmd_get() -> anyhow::Result<()> {
    todo!()
}

fn cmd_status() -> anyhow::Result<()> {
    todo!()
}

fn cmd_return(_path: Option<String>, _force: bool) -> anyhow::Result<()> {
    todo!()
}

fn cmd_destroy(_path: Option<String>, _force: bool, _all: bool) -> anyhow::Result<()> {
    todo!()
}

fn cmd_init() -> anyhow::Result<()> {
    todo!()
}

fn cmd_update() -> anyhow::Result<()> {
    todo!()
}
```

**Step 4: Build to verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

**Step 5: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat: add CLI skeleton with clap derive"
```

---

### Task 3: Git Module — Command Wrapper

**Files:**
- Create: `src/git.rs`
- Modify: `src/main.rs` (add `mod git;`)

This module wraps all git operations. Every function shells out to the system `git` binary and captures output.

**Step 1: Write tests for git helper functions**

Create `src/git.rs`:

```rust
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
    if let Ok(refname) = run_git(repo_root, &["symbolic-ref", "refs/remotes/origin/HEAD"]) {
        if let Some(branch) = refname.strip_prefix("refs/remotes/origin/") {
            return Ok(branch.to_string());
        }
    }

    // Try: git symbolic-ref HEAD
    if let Ok(refname) = run_git(repo_root, &["symbolic-ref", "HEAD"]) {
        if let Some(branch) = refname.strip_prefix("refs/heads/") {
            return Ok(branch.to_string());
        }
    }

    // Try: git config init.defaultBranch
    if let Ok(branch) = run_git(repo_root, &["config", "init.defaultBranch"]) {
        if !branch.is_empty() {
            return Ok(branch);
        }
    }

    bail!("could not determine default branch — try running: git remote set-head origin --auto")
}

pub fn remote_url(repo_root: &Path) -> Result<Option<String>> {
    // Check if origin remote exists
    let remotes = run_git(repo_root, &["remote"])?;
    if !remotes.lines().any(|r| r == "origin") {
        return Ok(None);
    }
    let url = run_git(repo_root, &["remote", "get-url", "origin"])?;
    Ok(Some(url))
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
    run_git(repo_root, &["worktree", "add", "--detach", path_str, ref_name])?;
    Ok(())
}

pub fn worktree_remove(repo_root: &Path, worktree_path: &Path) -> Result<()> {
    let path_str = worktree_path.to_str().context("invalid worktree path")?;
    run_git(repo_root, &["worktree", "remove", "--force", path_str])?;
    Ok(())
}

pub fn reset_worktree(worktree_path: &Path, ref_name: &str) -> Result<()> {
    run_git(worktree_path, &["checkout", "--detach", ref_name])?;
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
        Command::new("git").args(["init"]).current_dir(path).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(path).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test"]).current_dir(path).output().unwrap();
        // Create initial commit so HEAD exists
        std::fs::write(path.join("file.txt"), "hello").unwrap();
        Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
        Command::new("git").args(["commit", "-m", "init"]).current_dir(path).output().unwrap();
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
```

**Step 2: Add `mod git` and `tempfile` dev-dependency**

Add to `src/main.rs` after `mod cli;`:

```rust
mod git;
```

Add to `Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

**Step 3: Run tests**

Run: `cargo test --lib git::tests`
Expected: All 5 tests pass.

**Step 4: Commit**

```bash
git add src/git.rs src/main.rs Cargo.toml Cargo.lock
git commit -m "feat: add git module with command wrappers"
```

---

### Task 4: Config Module

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs` (add `mod config;`)

**Step 1: Write the config module with tests**

Create `src/config.rs`:

```rust
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::Deserialize;

const DEFAULT_MAX_TREES: usize = 16;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_max_trees")]
    pub max_trees: usize,
    #[serde(default)]
    pub root: String,
}

fn default_max_trees() -> usize {
    DEFAULT_MAX_TREES
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_trees: DEFAULT_MAX_TREES,
            root: String::new(),
        }
    }
}

impl Config {
    /// Load config from repo-level or user-level file. Returns defaults if neither exists.
    pub fn load(repo_root: &Path) -> Result<Self> {
        // 1. Repo-level: <repo_root>/tree-pool.toml
        let repo_config = repo_root.join("tree-pool.toml");
        if repo_config.exists() {
            let contents = std::fs::read_to_string(&repo_config)
                .with_context(|| format!("failed to read {}", repo_config.display()))?;
            let config: Config = toml::from_str(&contents)
                .with_context(|| format!("failed to parse {}", repo_config.display()))?;
            return Ok(config);
        }

        // 2. User-level: ~/.config/tree-pool/config.toml
        if let Some(config_dir) = dirs::config_dir() {
            let user_config = config_dir.join("tree-pool").join("config.toml");
            if user_config.exists() {
                let contents = std::fs::read_to_string(&user_config)
                    .with_context(|| format!("failed to read {}", user_config.display()))?;
                let config: Config = toml::from_str(&contents)
                    .with_context(|| format!("failed to parse {}", user_config.display()))?;
                return Ok(config);
            }
        }

        // No config found — use defaults
        Ok(Config::default())
    }

    /// Resolve the `root` config field to an absolute path.
    /// Empty string = home dir. Relative = relative to repo root. Supports env var expansion.
    pub fn resolve_root(&self, repo_root: &Path) -> Result<PathBuf> {
        if self.root.is_empty() {
            return dirs::home_dir().context("could not determine home directory");
        }

        let expanded = shellexpand::env(&self.root)
            .with_context(|| format!("failed to expand env vars in root: {}", self.root))?
            .to_string();

        let path = Path::new(&expanded);
        if path.is_absolute() {
            Ok(path.to_path_buf())
        } else {
            Ok(repo_root.join(path))
        }
    }

    /// Generate the default TOML content for `tp init`.
    pub fn default_toml() -> String {
        let config = Config::default();
        let mut content = toml::to_string_pretty(&config).unwrap_or_default();
        content.push_str("\n# root = \"\"  # Base directory for the worktree pool.\n");
        content.push_str("# Relative paths are relative to the repo root.\n");
        content.push_str("# Supports environment variables, e.g. \"$HOME/worktrees\".\n");
        content.push_str("# Default: home directory (~/.tree-pool/)\n");
        content
    }
}
```

Note: We need to add `serde::Serialize` derive and `shellexpand` dependency.

Update `Config` derive to include `Serialize`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
```

Add to `Cargo.toml` dependencies:

```toml
shellexpand = "3"
```

**Step 2: Add tests at the bottom of `src/config.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let config = Config::default();
        assert_eq!(config.max_trees, 16);
        assert_eq!(config.root, "");
    }

    #[test]
    fn parse_full_config() {
        let toml_str = r#"
            max_trees = 8
            root = "/tmp/worktrees"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.max_trees, 8);
        assert_eq!(config.root, "/tmp/worktrees");
    }

    #[test]
    fn parse_partial_config_uses_defaults() {
        let toml_str = r#"
            max_trees = 4
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.max_trees, 4);
        assert_eq!(config.root, "");
    }

    #[test]
    fn parse_empty_config_uses_defaults() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.max_trees, 16);
        assert_eq!(config.root, "");
    }

    #[test]
    fn resolve_root_empty_returns_home() {
        let config = Config::default();
        let root = config.resolve_root(Path::new("/repo")).unwrap();
        assert_eq!(root, dirs::home_dir().unwrap());
    }

    #[test]
    fn resolve_root_absolute() {
        let config = Config { max_trees: 16, root: "/tmp/custom".to_string() };
        let root = config.resolve_root(Path::new("/repo")).unwrap();
        assert_eq!(root, Path::new("/tmp/custom"));
    }

    #[test]
    fn resolve_root_relative() {
        let config = Config { max_trees: 16, root: "worktrees".to_string() };
        let root = config.resolve_root(Path::new("/repo")).unwrap();
        assert_eq!(root, Path::new("/repo/worktrees"));
    }

    #[test]
    fn load_returns_defaults_when_no_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert_eq!(config.max_trees, 16);
    }

    #[test]
    fn load_reads_repo_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("tree-pool.toml"), "max_trees = 3\n").unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert_eq!(config.max_trees, 3);
    }

    #[test]
    fn default_toml_is_valid() {
        let content = Config::default_toml();
        // The non-comment portion should parse
        let config: Config = toml::from_str(&content).unwrap();
        assert_eq!(config.max_trees, 16);
    }
}
```

**Step 3: Add `mod config` to main.rs**

Add after `mod git;`:

```rust
mod config;
```

**Step 4: Run tests**

Run: `cargo test --lib config::tests`
Expected: All 10 tests pass.

**Step 5: Commit**

```bash
git add src/config.rs src/main.rs Cargo.toml Cargo.lock
git commit -m "feat: add config module with TOML parsing"
```

---

### Task 5: State Module with File Locking

**Files:**
- Create: `src/state.rs`
- Modify: `src/main.rs` (add `mod state;`)

**Step 1: Write the state module with tests**

Create `src/state.rs`:

```rust
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeEntry {
    pub name: String,
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub worktrees: Vec<WorktreeEntry>,
}

/// Guard that holds the lock file open. Lock is released when dropped.
pub struct StateLock {
    _file: File,
}

impl State {
    /// Acquire an exclusive lock on the state lock file.
    pub fn lock(pool_dir: &Path) -> Result<StateLock> {
        fs::create_dir_all(pool_dir)
            .with_context(|| format!("failed to create pool dir {}", pool_dir.display()))?;

        let lock_path = pool_dir.join("tree-pool-state.lock");
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .with_context(|| format!("failed to open lock file {}", lock_path.display()))?;

        file.lock_exclusive()
            .context("failed to acquire state lock")?;

        Ok(StateLock { _file: file })
    }

    /// Read state from disk. Returns empty state if file doesn't exist.
    /// Heals stale entries (paths that no longer exist on disk).
    pub fn load(pool_dir: &Path) -> Result<Self> {
        let state_path = pool_dir.join("tree-pool-state.json");

        if !state_path.exists() {
            return Ok(State::default());
        }

        let contents = fs::read_to_string(&state_path)
            .with_context(|| format!("failed to read {}", state_path.display()))?;

        let mut state: State = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse {}", state_path.display()))?;

        // Self-healing: remove entries whose paths no longer exist
        let before = state.worktrees.len();
        state.worktrees.retain(|wt| wt.path.exists());
        if state.worktrees.len() != before {
            state.save(pool_dir)?;
        }

        Ok(state)
    }

    /// Write state to disk.
    pub fn save(&self, pool_dir: &Path) -> Result<()> {
        fs::create_dir_all(pool_dir)
            .with_context(|| format!("failed to create pool dir {}", pool_dir.display()))?;

        let state_path = pool_dir.join("tree-pool-state.json");
        let contents = serde_json::to_string_pretty(self)
            .context("failed to serialize state")?;

        fs::write(&state_path, contents)
            .with_context(|| format!("failed to write {}", state_path.display()))?;

        Ok(())
    }

    /// Find the next sequential worktree name (max existing + 1).
    pub fn next_name(&self) -> String {
        let max = self
            .worktrees
            .iter()
            .filter_map(|wt| wt.name.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        (max + 1).to_string()
    }

    /// Find a worktree entry by its absolute path.
    pub fn find_by_path(&self, path: &Path) -> Option<&WorktreeEntry> {
        self.worktrees.iter().find(|wt| wt.path == path)
    }

    /// Add a new worktree entry.
    pub fn add(&mut self, name: String, path: PathBuf) {
        self.worktrees.push(WorktreeEntry {
            name,
            path,
            created_at: Utc::now(),
        });
    }

    /// Remove a worktree entry by path.
    pub fn remove_by_path(&mut self, path: &Path) {
        self.worktrees.retain(|wt| wt.path != path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state_by_default() {
        let state = State::default();
        assert!(state.worktrees.is_empty());
    }

    #[test]
    fn next_name_empty_state() {
        let state = State::default();
        assert_eq!(state.next_name(), "1");
    }

    #[test]
    fn next_name_with_entries() {
        let mut state = State::default();
        state.add("1".to_string(), PathBuf::from("/a"));
        state.add("3".to_string(), PathBuf::from("/b"));
        assert_eq!(state.next_name(), "4");
    }

    #[test]
    fn find_by_path() {
        let mut state = State::default();
        state.add("1".to_string(), PathBuf::from("/a/b/c"));
        assert!(state.find_by_path(Path::new("/a/b/c")).is_some());
        assert!(state.find_by_path(Path::new("/x/y/z")).is_none());
    }

    #[test]
    fn remove_by_path() {
        let mut state = State::default();
        state.add("1".to_string(), PathBuf::from("/a"));
        state.add("2".to_string(), PathBuf::from("/b"));
        state.remove_by_path(Path::new("/a"));
        assert_eq!(state.worktrees.len(), 1);
        assert_eq!(state.worktrees[0].name, "2");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = State::default();
        state.add("1".to_string(), dir.path().to_path_buf()); // Use existing path so healing doesn't remove it
        state.save(dir.path()).unwrap();

        let loaded = State::load(dir.path()).unwrap();
        assert_eq!(loaded.worktrees.len(), 1);
        assert_eq!(loaded.worktrees[0].name, "1");
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = State::load(dir.path()).unwrap();
        assert!(state.worktrees.is_empty());
    }

    #[test]
    fn load_heals_stale_entries() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = State::default();
        state.add("1".to_string(), PathBuf::from("/nonexistent/path"));
        state.add("2".to_string(), dir.path().to_path_buf());
        // Write directly to avoid healing on save
        let state_path = dir.path().join("tree-pool-state.json");
        let contents = serde_json::to_string_pretty(&state).unwrap();
        fs::write(&state_path, contents).unwrap();

        let loaded = State::load(dir.path()).unwrap();
        assert_eq!(loaded.worktrees.len(), 1);
        assert_eq!(loaded.worktrees[0].name, "2");
    }

    #[test]
    fn lock_and_unlock() {
        let dir = tempfile::tempdir().unwrap();
        let _lock = State::lock(dir.path()).unwrap();
        // Lock is released when _lock is dropped
    }
}
```

**Step 2: Add `mod state` to main.rs**

Add after `mod config;`:

```rust
mod state;
```

**Step 3: Run tests**

Run: `cargo test --lib state::tests`
Expected: All 9 tests pass.

**Step 4: Commit**

```bash
git add src/state.rs src/main.rs
git commit -m "feat: add state module with JSON persistence and file locking"
```

---

### Task 6: Process Detection Module

**Files:**
- Create: `src/process.rs`
- Modify: `src/main.rs` (add `mod process;`)

**Step 1: Write the process detection module**

Create `src/process.rs`:

```rust
use std::path::Path;
use sysinfo::System;

#[derive(Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
}

/// Find all processes whose current working directory is inside the given path.
/// Uses proper path component checking (not string prefix).
pub fn processes_in_dir(dir: &Path) -> Vec<ProcessInfo> {
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let dir = match dir.canonicalize() {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let mut result = Vec::new();

    for (pid, process) in sys.processes() {
        let Some(cwd) = process.cwd() else {
            continue;
        };

        let cwd = match cwd.canonicalize() {
            Ok(c) => c,
            Err(_) => continue,
        };

        if cwd == dir || cwd.starts_with(&dir) {
            result.push(ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
            });
        }
    }

    result
}

/// Check if any process is using the given directory.
pub fn is_in_use(dir: &Path) -> bool {
    !processes_in_dir(dir).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_dir_is_detected() {
        let cwd = std::env::current_dir().unwrap();
        // The current process (this test) should be detected
        let procs = processes_in_dir(&cwd);
        assert!(!procs.is_empty(), "expected at least this test process");
    }

    #[test]
    fn nonexistent_dir_returns_empty() {
        let procs = processes_in_dir(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(procs.is_empty());
    }
}
```

**Step 2: Add `mod process` to main.rs**

Add after `mod state;`:

```rust
mod process;
```

**Step 3: Run tests**

Run: `cargo test --lib process::tests`
Expected: Both tests pass.

**Step 4: Commit**

```bash
git add src/process.rs src/main.rs
git commit -m "feat: add process detection module"
```

---

### Task 7: Pool Directory Resolution

**Files:**
- Create: `src/pool.rs`
- Modify: `src/main.rs` (add `mod pool;`)

**Step 1: Write the pool module**

Create `src/pool.rs`:

```rust
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use sha2::{Sha256, Digest};

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

/// Compute the worktree path within the pool.
/// Format: <poolDir>/<name>/<repoName>/
pub fn worktree_path(pool_dir: &Path, name: &str, repo_name: &str) -> PathBuf {
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
    fn worktree_path_format() {
        let pool_dir = Path::new("/home/user/.tree-pool/myrepo-abc123");
        let path = worktree_path(pool_dir, "1", "myrepo");
        assert_eq!(path, PathBuf::from("/home/user/.tree-pool/myrepo-abc123/1/myrepo"));
    }
}
```

Note: Add `hex` dependency to `Cargo.toml`:

```toml
hex = "0.4"
```

**Step 2: Add `mod pool` to main.rs**

Add after `mod process;`:

```rust
mod pool;
```

**Step 3: Run tests**

Run: `cargo test --lib pool::tests`
Expected: All 3 tests pass.

**Step 4: Commit**

```bash
git add src/pool.rs src/main.rs Cargo.toml Cargo.lock
git commit -m "feat: add pool directory resolution module"
```

---

### Task 8: Subshell Module

**Files:**
- Create: `src/shell.rs`
- Modify: `src/main.rs` (add `mod shell;`)

**Step 1: Write the shell module**

Create `src/shell.rs`:

```rust
use std::path::Path;
use std::process::Command;
use anyhow::{Context, Result};

/// Spawn a subshell in the given worktree directory.
/// Sets TREE_POOL_DIR in the environment.
/// Returns the shell's exit code.
pub fn spawn_subshell(worktree_path: &Path) -> Result<i32> {
    let shell = resolve_shell();

    let mut child = Command::new(&shell)
        .current_dir(worktree_path)
        .env("TREE_POOL_DIR", worktree_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn shell: {shell}"))?;

    let status = child.wait().context("failed to wait for shell")?;
    Ok(status.code().unwrap_or(1))
}

#[cfg(not(windows))]
fn resolve_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

#[cfg(windows)]
fn resolve_shell() -> String {
    std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
}

// No unit tests for this module — it spawns interactive shells.
// Tested via integration tests.
```

**Step 2: Add `mod shell` to main.rs**

Add after `mod pool;`:

```rust
mod shell;
```

**Step 3: Build to verify**

Run: `cargo build`
Expected: Compiles successfully.

**Step 4: Commit**

```bash
git add src/shell.rs src/main.rs
git commit -m "feat: add subshell spawning module"
```

---

### Task 9: Gitignore Module

**Files:**
- Create: `src/gitignore.rs`
- Modify: `src/main.rs` (add `mod gitignore;`)

**Step 1: Write the gitignore module with tests**

Create `src/gitignore.rs`:

```rust
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

use crate::git;

/// Ensure the given path is listed in the nearest .gitignore.
/// No-op if the path is outside any git repo.
/// Idempotent — checks for existing entry before appending.
pub fn ensure_ignored(pool_dir: &Path) -> Result<()> {
    // Find the nearest git repo containing the pool dir
    // Walk up from pool_dir to find an existing ancestor directory
    let existing_ancestor = pool_dir.ancestors()
        .find(|p| p.exists() && p.is_dir())
        .unwrap_or(pool_dir);

    let repo_root = match git::repo_root(existing_ancestor) {
        Ok(root) => root,
        Err(_) => return Ok(()), // Not inside a git repo — no-op
    };

    let repo_root = Path::new(&repo_root);

    // Compute the relative path from repo root to pool dir
    let rel_path = match pool_dir.strip_prefix(repo_root) {
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
        Cmd::new("git").args(["init"]).current_dir(path).output().unwrap();
        Cmd::new("git").args(["config", "user.email", "test@test.com"]).current_dir(path).output().unwrap();
        Cmd::new("git").args(["config", "user.name", "Test"]).current_dir(path).output().unwrap();
        std::fs::write(path.join("file.txt"), "hello").unwrap();
        Cmd::new("git").args(["add", "."]).current_dir(path).output().unwrap();
        Cmd::new("git").args(["commit", "-m", "init"]).current_dir(path).output().unwrap();
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
```

**Step 2: Add `mod gitignore` to main.rs**

Add after `mod shell;`:

```rust
mod gitignore;
```

**Step 3: Run tests**

Run: `cargo test --lib gitignore::tests`
Expected: All 3 tests pass.

**Step 4: Commit**

```bash
git add src/gitignore.rs src/main.rs
git commit -m "feat: add gitignore auto-management module"
```

---

### Task 10: Prompt Helper

**Files:**
- Create: `src/prompt.rs`
- Modify: `src/main.rs` (add `mod prompt;`)

**Step 1: Write the prompt module**

Create `src/prompt.rs`:

```rust
use std::io::{self, BufRead, Write};
use anyhow::{Result, bail};

/// Prompt the user with a yes/no question. Returns true for yes, false for no.
/// `default_yes` controls what happens when the user presses Enter without typing.
pub fn confirm(message: &str, default_yes: bool) -> Result<bool> {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    eprint!("{message} {suffix} ");
    io::stderr().flush()?;

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input.is_empty() {
        return Ok(default_yes);
    }

    match input.as_str() {
        "y" | "yes" => Ok(true),
        "n" | "no" => Ok(false),
        _ => bail!("invalid input: {input}"),
    }
}

// No unit tests — interactive I/O. Tested manually / via integration tests.
```

**Step 2: Add `mod prompt` to main.rs**

Add after `mod gitignore;`:

```rust
mod prompt;
```

**Step 3: Build to verify**

Run: `cargo build`
Expected: Compiles successfully.

**Step 4: Commit**

```bash
git add src/prompt.rs src/main.rs
git commit -m "feat: add user prompt helper"
```

---

### Task 11: Implement `tp init` Command

**Files:**
- Modify: `src/main.rs`

**Step 1: Implement `cmd_init`**

Replace the `cmd_init` function in `src/main.rs`:

```rust
fn cmd_init() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let root = git::repo_root(&cwd)?;
    let config_path = Path::new(&root).join("tree-pool.toml");

    if config_path.exists() {
        anyhow::bail!("tree-pool.toml already exists at {}", config_path.display());
    }

    let content = config::Config::default_toml();
    std::fs::write(&config_path, content)?;
    eprintln!("created {}", config_path.display());
    Ok(())
}
```

Add at the top of main.rs:

```rust
use std::path::Path;
```

**Step 2: Build to verify**

Run: `cargo build`
Expected: Compiles successfully.

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement tp init command"
```

---

### Task 12: Implement `tp update` Command

**Files:**
- Modify: `src/main.rs`

**Step 1: Implement `cmd_update`**

Replace the `cmd_update` function in `src/main.rs`:

```rust
fn cmd_update() -> anyhow::Result<()> {
    eprintln!("updating tree-pool...");
    let status = std::process::Command::new("cargo")
        .args(["install", "tree-pool"])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        anyhow::bail!("cargo install tree-pool failed");
    }

    Ok(())
}
```

**Step 2: Build to verify**

Run: `cargo build`
Expected: Compiles successfully.

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement tp update command"
```

---

### Task 13: Implement `tp status` Command

**Files:**
- Create: `src/display.rs`
- Modify: `src/main.rs`

**Step 1: Write display helpers**

Create `src/display.rs`:

```rust
use std::path::{Path, PathBuf};

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
```

**Step 2: Implement `cmd_status`**

Replace `cmd_status` in `src/main.rs`:

```rust
fn cmd_status() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let root = git::repo_root(&cwd)?;
    let repo_root = Path::new(&root);
    let config = config::Config::load(repo_root)?;
    let pool_dir = pool::resolve_pool_dir(repo_root, &config)?;

    let _lock = state::State::lock(&pool_dir)?;
    let state = state::State::load(&pool_dir)?;

    if state.worktrees.is_empty() {
        eprintln!("no worktrees in pool");
        return Ok(());
    }

    use colored::Colorize;

    for wt in &state.worktrees {
        let procs = process::processes_in_dir(&wt.path);
        let dirty = git::is_dirty(&wt.path).unwrap_or(false);
        let current = display::is_current_dir(&wt.path);

        let (status_str, status_colored) = if current {
            ("here", "here".cyan().bold().to_string())
        } else if !procs.is_empty() {
            ("in-use", "in-use".red().to_string())
        } else if dirty {
            ("dirty", "dirty".yellow().to_string())
        } else {
            ("available", "available".green().to_string())
        };

        let _ = status_str; // suppress unused warning
        println!(
            "{:>4}  {:<11}  {}",
            wt.name,
            status_colored,
            display::pretty_path(&wt.path)
        );

        if !procs.is_empty() && !current {
            let proc_list: Vec<String> = procs
                .iter()
                .map(|p| format!("{} ({})", p.name, p.pid))
                .collect();
            println!("        {}", proc_list.join(", "));
        }
    }

    Ok(())
}
```

Add `mod display;` to main.rs after `mod prompt;`.

**Step 3: Build to verify**

Run: `cargo build`
Expected: Compiles successfully.

**Step 4: Commit**

```bash
git add src/display.rs src/main.rs
git commit -m "feat: implement tp status command"
```

---

### Task 14: Implement `tp get` Command

**Files:**
- Modify: `src/main.rs`

This is the core command — acquire a worktree and spawn a subshell.

**Step 1: Implement `cmd_get`**

Replace `cmd_get` in `src/main.rs`:

```rust
fn cmd_get() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let root = git::repo_root(&cwd)?;
    let repo_root_path = PathBuf::from(&root);
    let config = config::Config::load(&repo_root_path)?;
    let pool_dir = pool::resolve_pool_dir(&repo_root_path, &config)?;

    let repo_name = repo_root_path
        .file_name()
        .context("repo root has no name")?
        .to_string_lossy()
        .to_string();

    // Ensure .gitignore covers the pool dir
    if let Err(e) = gitignore::ensure_ignored(&pool_dir) {
        eprintln!("warning: failed to update .gitignore: {e}");
    }

    let _lock = state::State::lock(&pool_dir)?;
    let mut st = state::State::load(&pool_dir)?;

    // Try to find an available worktree (not in-use and not dirty)
    let available = st.worktrees.iter().find(|wt| {
        !process::is_in_use(&wt.path) && !git::is_dirty(&wt.path).unwrap_or(true)
    });

    let wt_path = if let Some(wt) = available {
        let wt_path = wt.path.clone();
        // Reset to latest default branch
        eprintln!("fetching origin...");
        git::fetch_origin(&repo_root_path)?;
        let branch = git::default_branch(&repo_root_path)?;
        let ref_name = git::branch_ref(&repo_root_path, &branch)?;
        git::reset_worktree(&wt_path, &ref_name)?;
        eprintln!("reusing worktree: {}", display::pretty_path(&wt_path));
        wt_path
    } else if st.worktrees.len() < config.max_trees {
        // Create a new worktree
        eprintln!("fetching origin...");
        git::fetch_origin(&repo_root_path)?;
        let branch = git::default_branch(&repo_root_path)?;
        let ref_name = git::branch_ref(&repo_root_path, &branch)?;

        let name = st.next_name();
        let wt_path = pool::worktree_path(&pool_dir, &name, &repo_name);

        // Create parent dir
        if let Some(parent) = wt_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        git::worktree_add(&repo_root_path, &wt_path, &ref_name)?;
        st.add(name, wt_path.clone());
        st.save(&pool_dir)?;
        eprintln!("created worktree: {}", display::pretty_path(&wt_path));
        wt_path
    } else {
        anyhow::bail!(
            "all {} worktrees are in use or dirty — run `tp status` to see details, \
             `tp return` to return a dirty worktree, or increase max_trees in tree-pool.toml",
            config.max_trees
        );
    };

    // Drop the lock before spawning the subshell
    drop(_lock);

    let exit_code = shell::spawn_subshell(&wt_path)?;

    // On exit, check if dirty and prompt
    if git::is_dirty(&wt_path).unwrap_or(false) {
        if prompt::confirm("worktree has uncommitted changes. return it anyway?", true).unwrap_or(true) {
            let branch = git::default_branch(&repo_root_path)?;
            let ref_name = git::branch_ref(&repo_root_path, &branch)?;
            if let Err(e) = git::reset_worktree(&wt_path, &ref_name) {
                eprintln!("warning: failed to reset worktree: {e}");
            }
        }
    } else {
        // Clean exit — release the worktree
        let branch = git::default_branch(&repo_root_path)?;
        let ref_name = git::branch_ref(&repo_root_path, &branch)?;
        if let Err(e) = git::reset_worktree(&wt_path, &ref_name) {
            eprintln!("warning: failed to reset worktree: {e}");
        }
    }

    std::process::exit(exit_code);
}
```

Add to the top of `main.rs`:

```rust
use std::path::PathBuf;
use anyhow::Context;
```

**Step 2: Build to verify**

Run: `cargo build`
Expected: Compiles successfully.

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement tp get command"
```

---

### Task 15: Implement `tp return` Command

**Files:**
- Modify: `src/main.rs`

**Step 1: Implement `cmd_return`**

Replace `cmd_return` in `src/main.rs`:

```rust
fn cmd_return(path: Option<String>, force: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let root = git::repo_root(&cwd)?;
    let repo_root_path = PathBuf::from(&root);
    let config = config::Config::load(&repo_root_path)?;
    let pool_dir = pool::resolve_pool_dir(&repo_root_path, &config)?;

    // Resolve the worktree path
    let wt_path = if let Some(p) = path {
        PathBuf::from(p).canonicalize()?
    } else if let Ok(env_path) = std::env::var("TREE_POOL_DIR") {
        PathBuf::from(env_path).canonicalize()?
    } else {
        cwd.canonicalize()?
    };

    let _lock = state::State::lock(&pool_dir)?;
    let st = state::State::load(&pool_dir)?;

    // Validate this is a known worktree
    if st.find_by_path(&wt_path).is_none() {
        anyhow::bail!("{} is not a tree-pool worktree", wt_path.display());
    }

    // Check dirty
    if git::is_dirty(&wt_path)? && !force {
        if !prompt::confirm("worktree has uncommitted changes. return it anyway?", true)? {
            return Ok(());
        }
    }

    // Reset to clean state
    let branch = git::default_branch(&repo_root_path)?;
    let ref_name = git::branch_ref(&repo_root_path, &branch)?;
    git::reset_worktree(&wt_path, &ref_name)?;
    eprintln!("returned {}", display::pretty_path(&wt_path));
    Ok(())
}
```

**Step 2: Build to verify**

Run: `cargo build`
Expected: Compiles successfully.

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement tp return command"
```

---

### Task 16: Implement `tp destroy` Command

**Files:**
- Modify: `src/main.rs`

**Step 1: Implement `cmd_destroy`**

Replace `cmd_destroy` in `src/main.rs`:

```rust
fn cmd_destroy(path: Option<String>, force: bool, all: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let root = git::repo_root(&cwd)?;
    let repo_root_path = PathBuf::from(&root);
    let config = config::Config::load(&repo_root_path)?;
    let pool_dir = pool::resolve_pool_dir(&repo_root_path, &config)?;

    let _lock = state::State::lock(&pool_dir)?;
    let mut st = state::State::load(&pool_dir)?;

    if all {
        if st.worktrees.is_empty() {
            eprintln!("no worktrees to destroy");
            return Ok(());
        }

        if !force {
            if !prompt::confirm(
                &format!("destroy all {} worktrees?", st.worktrees.len()),
                false,
            )? {
                return Ok(());
            }
        }

        let paths: Vec<_> = st.worktrees.iter().map(|wt| wt.path.clone()).collect();
        for wt_path in &paths {
            if !force && process::is_in_use(wt_path) {
                eprintln!(
                    "skipping {} (in use) — use --force to override",
                    display::pretty_path(wt_path)
                );
                continue;
            }
            destroy_worktree(&repo_root_path, &pool_dir, wt_path, &mut st)?;
        }
    } else {
        let path = path.context("path argument is required (or use --all)")?;
        let wt_path = PathBuf::from(&path).canonicalize()?;

        if st.find_by_path(&wt_path).is_none() {
            anyhow::bail!("{} is not a tree-pool worktree", wt_path.display());
        }

        if !force {
            if process::is_in_use(&wt_path) {
                anyhow::bail!(
                    "{} is in use — use --force to override",
                    display::pretty_path(&wt_path)
                );
            }

            if !prompt::confirm(
                &format!("destroy worktree {}?", display::pretty_path(&wt_path)),
                false,
            )? {
                return Ok(());
            }
        }

        destroy_worktree(&repo_root_path, &pool_dir, &wt_path, &mut st)?;
    }

    st.save(&pool_dir)?;
    Ok(())
}

fn destroy_worktree(
    repo_root: &Path,
    _pool_dir: &Path,
    wt_path: &Path,
    st: &mut state::State,
) -> anyhow::Result<()> {
    // Remove git worktree
    let _ = git::worktree_remove(repo_root, wt_path);

    // Remove the numbered parent directory (e.g., <poolDir>/1/)
    if let Some(parent) = wt_path.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }

    st.remove_by_path(wt_path);
    eprintln!("destroyed {}", display::pretty_path(wt_path));
    Ok(())
}
```

**Step 2: Build to verify**

Run: `cargo build`
Expected: Compiles successfully.

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement tp destroy command"
```

---

### Task 17: Final Polish and README

**Files:**
- Modify: `README.md`

**Step 1: Update README with usage docs and credits**

Replace `README.md`:

```markdown
# tree-pool

A CLI tool (`tp`) for managing pools of reusable, pre-warmed git worktrees for parallel AI coding agent workflows.

## Install

```
cargo install tree-pool
```

## Usage

```bash
tp              # Acquire a worktree and open a subshell (alias for `tp get`)
tp get          # Same as above
tp status       # Show pool status
tp return       # Return a worktree to the pool
tp destroy      # Remove a worktree permanently
tp init         # Create tree-pool.toml in repo root
tp update       # Update tree-pool via cargo install
```

## Configuration

Create `tree-pool.toml` in your repo root (or `~/.config/tree-pool/config.toml` for global config):

```toml
max_trees = 16
# root = ""  # Base directory for the pool (default: home directory)
```

## Credits

This project is a Rust port of [treehouse](https://github.com/kunchenguid/treehouse) by [kunchenguid](https://github.com/kunchenguid).
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add README with usage and credits"
```

---

### Task 18: Integration Smoke Test

**Files:**
- Create: `tests/integration.rs`

**Step 1: Write a basic integration test**

Create `tests/integration.rs`:

```rust
use std::process::Command;

fn tp() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tp"))
}

#[test]
fn version_flag() {
    let output = tp().arg("--version").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tree-pool"));
}

#[test]
fn init_creates_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    // Set up a git repo
    Command::new("git").args(["init"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.name", "Test"]).current_dir(path).output().unwrap();
    std::fs::write(path.join("file.txt"), "hello").unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git").args(["commit", "-m", "init"]).current_dir(path).output().unwrap();

    let output = tp().arg("init").current_dir(path).output().unwrap();
    assert!(output.status.success(), "tp init failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(path.join("tree-pool.toml").exists());

    // Second init should fail
    let output = tp().arg("init").current_dir(path).output().unwrap();
    assert!(!output.status.success());
}

#[test]
fn status_in_empty_pool() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    Command::new("git").args(["init"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.name", "Test"]).current_dir(path).output().unwrap();
    std::fs::write(path.join("file.txt"), "hello").unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git").args(["commit", "-m", "init"]).current_dir(path).output().unwrap();

    let output = tp().arg("status").current_dir(path).output().unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no worktrees"));
}
```

**Step 2: Run integration tests**

Run: `cargo test --test integration`
Expected: All 3 tests pass.

**Step 3: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add integration smoke tests"
```
