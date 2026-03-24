# Branch Selection & .meta Directory Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use cipherpowers:executing-plans to implement this plan task-by-task.

**Goal:** Let `tp get` accept/prompt for a branch name via interactive selector, and move pool metadata into a `.meta/` subdirectory.

**Architecture:** Add `dialoguer` for interactive branch picking. New git helpers list branches sorted by commit date and detect which are already checked out. State paths move from `<pool>/` to `<pool>/.meta/` with automatic migration.

**Tech Stack:** Rust, clap, dialoguer, colored, git CLI

---

### Task 1: Add `dialoguer` dependency

**Files:**
- Modify: `Cargo.toml:12-26`

**Step 1: Add the dependency**

In `Cargo.toml`, add `dialoguer` to `[dependencies]`:

```toml
dialoguer = "0.11"
```

Add it after the `colored` line (line 17).

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add dialoguer dependency for interactive branch selection"
```

---

### Task 2: Move state paths to `.meta/` subdirectory

**Files:**
- Modify: `src/state.rs:27-37` (lock path), `src/state.rs:48-55` (load path), `src/state.rs:71-79` (save path)
- Test: `src/state.rs` (existing tests + new migration test)

**Step 1: Write failing test for `.meta` directory usage**

Add to `src/state.rs` in the `mod tests` block:

```rust
#[test]
fn lock_creates_meta_dir() {
    let dir = tempfile::tempdir().unwrap();
    let _lock = State::lock(dir.path()).unwrap();
    assert!(dir.path().join(".meta").exists());
    assert!(dir.path().join(".meta").join("tree-pool-state.lock").exists());
}

#[test]
fn save_writes_to_meta_dir() {
    let dir = tempfile::tempdir().unwrap();
    let state = State::default();
    state.save(dir.path()).unwrap();
    assert!(dir.path().join(".meta").join("tree-pool-state.json").exists());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib state::tests::lock_creates_meta_dir state::tests::save_writes_to_meta_dir`
Expected: FAIL — lock and state files are still created at pool root, not `.meta/`

**Step 3: Add `meta_dir` helper and update `lock`, `load`, `save`**

Add a helper function inside `impl State` (before `lock`):

```rust
/// Return the .meta subdirectory path within the pool dir.
fn meta_dir(pool_dir: &Path) -> PathBuf {
    pool_dir.join(".meta")
}
```

Update `lock` to use `.meta`:

```rust
pub fn lock(pool_dir: &Path) -> Result<StateLock> {
    let meta = Self::meta_dir(pool_dir);
    fs::create_dir_all(&meta)
        .with_context(|| format!("failed to create meta dir {}", meta.display()))?;

    let lock_path = meta.join("tree-pool-state.lock");
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
```

Update `load` — change state path and add migration:

```rust
pub fn load(pool_dir: &Path) -> Result<Self> {
    let meta = Self::meta_dir(pool_dir);
    Self::migrate_to_meta(pool_dir, &meta)?;

    let state_path = meta.join("tree-pool-state.json");

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
```

Update `save`:

```rust
pub fn save(&self, pool_dir: &Path) -> Result<()> {
    let meta = Self::meta_dir(pool_dir);
    fs::create_dir_all(&meta)
        .with_context(|| format!("failed to create meta dir {}", meta.display()))?;

    let state_path = meta.join("tree-pool-state.json");
    let contents = serde_json::to_string_pretty(self).context("failed to serialize state")?;

    fs::write(&state_path, contents)
        .with_context(|| format!("failed to write {}", state_path.display()))?;

    Ok(())
}
```

Add the migration function inside `impl State`:

```rust
/// Migrate old state/lock files from pool root to .meta/ subdirectory.
fn migrate_to_meta(pool_dir: &Path, meta: &Path) -> Result<()> {
    let old_state = pool_dir.join("tree-pool-state.json");
    let old_lock = pool_dir.join("tree-pool-state.lock");

    if !old_state.exists() && !old_lock.exists() {
        return Ok(());
    }
    if meta.join("tree-pool-state.json").exists() {
        // Already migrated — clean up old files if they linger
        let _ = fs::remove_file(&old_state);
        let _ = fs::remove_file(&old_lock);
        return Ok(());
    }

    fs::create_dir_all(meta)
        .with_context(|| format!("failed to create meta dir {}", meta.display()))?;

    if old_state.exists() {
        fs::rename(&old_state, meta.join("tree-pool-state.json"))
            .context("failed to migrate state file to .meta/")?;
    }
    if old_lock.exists() {
        let _ = fs::remove_file(&old_lock); // Lock files are ephemeral, just delete
    }

    Ok(())
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib state::tests`
Expected: ALL state tests pass (including existing ones, which now use `.meta/` transparently)

**Step 5: Write migration test**

Add to `mod tests`:

```rust
#[test]
fn migrate_moves_old_files_to_meta() {
    let dir = tempfile::tempdir().unwrap();
    let pool = dir.path();

    // Create old-style files at pool root
    fs::write(pool.join("tree-pool-state.json"), r#"{"worktrees":[]}"#).unwrap();
    fs::write(pool.join("tree-pool-state.lock"), "").unwrap();

    // Load triggers migration
    let state = State::load(pool).unwrap();
    assert!(state.worktrees.is_empty());

    // Old files should be gone
    assert!(!pool.join("tree-pool-state.json").exists());
    assert!(!pool.join("tree-pool-state.lock").exists());

    // New files should exist in .meta/
    assert!(pool.join(".meta").join("tree-pool-state.json").exists());
}
```

**Step 6: Run migration test**

Run: `cargo test --lib state::tests::migrate_moves_old_files_to_meta`
Expected: PASS

**Step 7: Run full test suite**

Run: `cargo test`
Expected: ALL tests pass

**Step 8: Commit**

```bash
git add src/state.rs
git commit -m "refactor: move pool metadata into .meta/ subdirectory"
```

---

### Task 3: Add git branch listing and checkout detection

**Files:**
- Modify: `src/git.rs`
- Test: `src/git.rs` (new tests in existing `mod tests`)

**Step 1: Write failing tests**

Add to `src/git.rs` in `mod tests`:

```rust
#[test]
fn test_list_branches_by_date() {
    let dir = setup_test_repo();
    let branches = list_branches_by_date(dir.path()).unwrap();
    // Should contain at least the default branch
    assert!(!branches.is_empty());
}

#[test]
fn test_checked_out_branches_includes_head() {
    let dir = setup_test_repo();
    let branches = checked_out_branches(dir.path()).unwrap();
    // The main repo has a branch checked out
    assert!(!branches.is_empty());
}

#[test]
fn test_current_branch_detached() {
    let dir = setup_test_repo();
    // Detach HEAD
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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib git::tests::test_list_branches git::tests::test_checked_out git::tests::test_current_branch`
Expected: FAIL — functions don't exist yet

**Step 3: Implement the functions**

Add to `src/git.rs` (before the `#[cfg(test)]` block):

```rust
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
        // Normalize: strip "origin/" prefix for deduplication
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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib git::tests`
Expected: ALL git tests pass

**Step 5: Commit**

```bash
git add src/git.rs
git commit -m "feat: add branch listing, checkout detection, and current branch helpers"
```

---

### Task 4: Add interactive branch selector

**Files:**
- Create: `src/branch.rs`
- Modify: `src/main.rs:1` (add `mod branch;`)

**Step 1: Create the branch selector module**

Create `src/branch.rs`:

```rust
use std::path::Path;

use anyhow::{Result, bail};
use colored::Colorize;
use dialoguer::{Select, Input, theme::ColorfulTheme};

use crate::git;

const CREATE_NEW_LABEL: &str = "[ Create new branch ]";

/// Prompt the user to select a branch interactively or create a new one.
/// Returns the branch name and whether it's a new branch.
pub fn select_branch(repo_root: &Path) -> Result<(String, bool)> {
    let all_branches = git::list_branches_by_date(repo_root)?;
    let checked_out = git::checked_out_branches(repo_root)?;

    // Build display items: first is "create new", rest are branches
    let mut items: Vec<String> = vec![CREATE_NEW_LABEL.to_string()];
    let mut selectable: Vec<bool> = vec![true];

    for branch in &all_branches {
        let is_checked_out = checked_out.contains(branch);
        if is_checked_out {
            items.push(format!("{}", branch.dimmed()));
        } else {
            items.push(branch.clone());
        }
        selectable.push(!is_checked_out);
    }

    if items.len() == 1 {
        // No branches to show, go straight to create
        return prompt_new_branch(&all_branches);
    }

    loop {
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("select a branch")
            .items(&items)
            .default(0)
            .interact()?;

        if !selectable[selection] {
            eprintln!("that branch is already checked out in another worktree");
            continue;
        }

        if selection == 0 {
            return prompt_new_branch(&all_branches);
        }

        return Ok((all_branches[selection - 1].clone(), false));
    }
}

/// Validate a branch name provided via CLI argument.
/// Returns error if the branch is already checked out.
pub fn validate_branch(repo_root: &Path, branch: &str) -> Result<bool> {
    let checked_out = git::checked_out_branches(repo_root)?;
    if checked_out.contains(branch) {
        bail!("branch '{branch}' is already checked out in another worktree");
    }

    // Check if branch exists locally or on remote
    let exists = git::branch_exists(repo_root, branch)?;
    Ok(exists)
}

fn prompt_new_branch(existing: &[String]) -> Result<(String, bool)> {
    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("new branch name")
        .interact_text()?;

    let name = name.trim().to_string();
    if name.is_empty() {
        bail!("branch name cannot be empty");
    }

    if existing.contains(&name) {
        bail!("branch '{name}' already exists — select it from the list instead");
    }

    Ok((name, true))
}
```

**Step 2: Add `branch_exists` helper to `src/git.rs`**

Add before the `#[cfg(test)]` block in `src/git.rs`:

```rust
/// Check if a branch exists locally or on a remote.
pub fn branch_exists(repo_root: &Path, branch: &str) -> Result<bool> {
    let local = format!("refs/heads/{branch}");
    let remote = format!("refs/remotes/origin/{branch}");
    let local_exists = run_git_ok(repo_root, &["rev-parse", "--verify", &local])?;
    let remote_exists = run_git_ok(repo_root, &["rev-parse", "--verify", &remote])?;
    Ok(local_exists || remote_exists)
}
```

**Step 3: Add `mod branch;` to `src/main.rs`**

Add `mod branch;` after the existing `mod` declarations at the top of `src/main.rs` (after line 1):

```rust
mod branch;
```

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

**Step 5: Add test for `branch_exists`**

Add to `src/git.rs` in `mod tests`:

```rust
#[test]
fn test_branch_exists() {
    let dir = setup_test_repo();
    let branch = default_branch(dir.path()).unwrap();
    assert!(branch_exists(dir.path(), &branch).unwrap());
    assert!(!branch_exists(dir.path(), "nonexistent-branch-xyz").unwrap());
}
```

**Step 6: Run tests**

Run: `cargo test --lib git::tests::test_branch_exists`
Expected: PASS

**Step 7: Commit**

```bash
git add src/branch.rs src/git.rs src/main.rs
git commit -m "feat: add interactive branch selector with dialoguer"
```

---

### Task 5: Add non-detached worktree creation to git module

**Files:**
- Modify: `src/git.rs`

**Step 1: Write failing test**

Add to `src/git.rs` in `mod tests`:

```rust
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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib git::tests::test_worktree_add_new git::tests::test_worktree_add_existing`
Expected: FAIL — functions don't exist yet

**Step 3: Implement**

Add to `src/git.rs` before `#[cfg(test)]`:

```rust
/// Create a worktree with a new branch.
pub fn worktree_add_new_branch(repo_root: &Path, worktree_path: &Path, branch: &str) -> Result<()> {
    let path_str = worktree_path.to_str().context("invalid worktree path")?;
    run_git(repo_root, &["worktree", "add", "-b", branch, path_str])?;
    Ok(())
}

/// Create a worktree checking out an existing branch.
pub fn worktree_add_existing_branch(repo_root: &Path, worktree_path: &Path, branch: &str) -> Result<()> {
    let path_str = worktree_path.to_str().context("invalid worktree path")?;
    run_git(repo_root, &["worktree", "add", path_str, branch])?;
    Ok(())
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib git::tests::test_worktree_add_new git::tests::test_worktree_add_existing`
Expected: PASS

**Step 5: Commit**

```bash
git add src/git.rs
git commit -m "feat: add worktree creation helpers for new and existing branches"
```

---

### Task 6: Add branch argument to CLI

**Files:**
- Modify: `src/cli.rs:17` (change `Get` variant)

**Step 1: Write failing test**

Add to `src/cli.rs` in `mod tests`:

```rust
#[test]
fn get_with_branch() {
    let cli = Cli::parse_from(["tp", "get", "feature/foo"]);
    match cli.command {
        Some(Command::Get { branch }) => {
            assert_eq!(branch.as_deref(), Some("feature/foo"));
        }
        _ => panic!("expected Get command"),
    }
}

#[test]
fn get_without_branch() {
    let cli = Cli::parse_from(["tp", "get"]);
    match cli.command {
        Some(Command::Get { branch }) => {
            assert!(branch.is_none());
        }
        _ => panic!("expected Get command"),
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib cli::tests::get_with_branch cli::tests::get_without_branch`
Expected: FAIL — `Get` has no fields

**Step 3: Update `Get` variant**

In `src/cli.rs`, change the `Get` variant (line 17) from:

```rust
    Get,
```

to:

```rust
    Get {
        /// Branch to check out in the worktree
        branch: Option<String>,
    },
```

Update the existing `get_subcommand` test to match the new shape:

```rust
#[test]
fn get_subcommand() {
    let cli = Cli::parse_from(["tp", "get"]);
    assert!(matches!(cli.command, Some(Command::Get { .. })));
}
```

Update `src/main.rs` line 22 to destructure the new field:

```rust
None | Some(Command::Get { branch }) => cmd_get(branch),
```

Update `cmd_get` signature on line 36:

```rust
fn cmd_get(branch: Option<String>) -> anyhow::Result<()> {
```

Also update the `None` arm — when no subcommand is provided, pass `None`:

Change line 22 from:
```rust
None | Some(Command::Get { branch }) => cmd_get(branch),
```
to:
```rust
Some(Command::Get { branch }) => cmd_get(branch),
None => cmd_get(None),
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib cli::tests`
Expected: ALL cli tests pass

**Step 5: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat: add optional branch argument to tp get"
```

---

### Task 7: Rework `cmd_get` for branch selection

This is the core integration task. It rewires `cmd_get` to use branch selection.

**Files:**
- Modify: `src/main.rs:36-108` (the `cmd_get` function)

**Step 1: Rewrite `cmd_get`**

Replace the entire `cmd_get` function in `src/main.rs`:

```rust
fn cmd_get(branch: Option<String>) -> anyhow::Result<()> {
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

    // Fetch before branch selection so the list is up to date
    if git::has_origin(&repo_root_path)? {
        eprintln!("fetching origin...");
        if let Err(e) = git::fetch_origin(&repo_root_path) {
            eprintln!("warning: failed to fetch origin: {e}");
        }
    }

    // Resolve which branch to use
    let (selected_branch, is_new) = if let Some(ref b) = branch {
        let exists = branch::validate_branch(&repo_root_path, b)?;
        (b.clone(), !exists)
    } else {
        branch::select_branch(&repo_root_path)?
    };

    let _lock = state::State::lock(&pool_dir)?;
    let mut st = state::State::load(&pool_dir)?;

    // Try to find an available worktree (not in-use and not dirty)
    let available = st
        .worktrees
        .iter()
        .find(|wt| !process::is_in_use(&wt.path) && !git::is_dirty(&wt.path).unwrap_or(true));

    let wt_path = if let Some(wt) = available {
        let wt_path = wt.path.clone();
        if is_new {
            // Create a new branch at the worktree's current HEAD, then switch
            let default = git::default_branch(&repo_root_path)?;
            let ref_name = git::branch_ref(&repo_root_path, &default)?;
            git::reset_worktree(&wt_path, &ref_name)?;
            git::create_and_checkout_branch(&wt_path, &selected_branch)?;
        } else {
            git::checkout_branch(&wt_path, &selected_branch)?;
        }
        eprintln!("reusing worktree: {}", display::pretty_path(&wt_path));
        wt_path
    } else if st.worktrees.len() < config.max_trees {
        let name = st.next_name();
        let wt_path = pool::worktree_path(&pool_dir, &name, &repo_name);

        if let Some(parent) = wt_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if is_new {
            git::worktree_add_new_branch(&repo_root_path, &wt_path, &selected_branch)?;
        } else {
            git::worktree_add_existing_branch(&repo_root_path, &wt_path, &selected_branch)?;
        }
        let wt_path = wt_path.canonicalize().unwrap_or(wt_path);
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

    eprintln!("on branch: {selected_branch}");
    let exit_code = shell::spawn_subshell(&wt_path)?;

    // On exit, return worktree to clean detached state on default branch
    let default = git::default_branch(&repo_root_path)?;
    let ref_name = git::branch_ref(&repo_root_path, &default)?;

    if git::is_dirty(&wt_path).unwrap_or(false) {
        if prompt::confirm("worktree has uncommitted changes. return it anyway?", false)
            .unwrap_or(true)
        {
            if let Err(e) = git::reset_worktree(&wt_path, &ref_name) {
                eprintln!("warning: failed to reset worktree: {e}");
            }
        }
    } else {
        if let Err(e) = git::reset_worktree(&wt_path, &ref_name) {
            eprintln!("warning: failed to reset worktree: {e}");
        }
    }

    std::process::exit(exit_code);
}
```

**Step 2: Add the two new git helpers**

Add to `src/git.rs` before `#[cfg(test)]`:

```rust
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
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

**Step 4: Run full test suite**

Run: `cargo test`
Expected: ALL tests pass

**Step 5: Commit**

```bash
git add src/main.rs src/git.rs
git commit -m "feat: rework cmd_get to use branch selection"
```

---

### Task 8: Show branch name in `tp status`

**Files:**
- Modify: `src/main.rs:138-186` (the `cmd_status` function)

**Step 1: Update status display**

In `cmd_status`, update the format string in the `for` loop. After the status detection block (line 168), add branch detection and update the `println!`:

Replace the `println!` and preceding status logic (lines 155-174) with:

```rust
    for wt in &state.worktrees {
        let procs = process::processes_in_dir(&wt.path);
        let dirty = git::is_dirty(&wt.path).unwrap_or(false);
        let current = display::is_current_dir(&wt.path);

        let status_colored = if current {
            "here".cyan().bold().to_string()
        } else if !procs.is_empty() {
            "in-use".red().to_string()
        } else if dirty {
            "dirty".yellow().to_string()
        } else {
            "available".green().to_string()
        };

        let branch = git::current_branch(&wt.path)
            .unwrap_or(None)
            .unwrap_or_else(|| "(detached)".to_string());

        println!(
            "{:>4}  {:<11}  {:<20}  {}",
            wt.name,
            status_colored,
            branch,
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
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

**Step 3: Run full test suite**

Run: `cargo test`
Expected: ALL tests pass

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: show branch name in tp status output"
```

---

### Task 9: Update README

**Files:**
- Modify: `README.md`

**Step 1: Update usage section**

Update the usage block in `README.md` to reflect the new branch argument:

```bash
tp                    # Acquire a worktree (prompts for branch)
tp get                # Same as above
tp get <branch>       # Acquire a worktree on a specific branch
tp status             # Show pool status (includes branch names)
tp return             # Return a worktree to the pool
tp destroy            # Remove a worktree permanently
tp init               # Create tree-pool.toml in repo root
tp update             # Update tree-pool via cargo install
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: update README with branch selection usage"
```
