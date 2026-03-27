# Architecture

## Overview

tree-pool is a Rust CLI tool that manages pools of pre-warmed, reusable local git clones for parallel AI coding agent workflows. Each clone uses `git clone --local` (hardlinked object stores, near-zero disk overhead) and gets two remotes: `origin` (real upstream) and `local` (source repo on disk).

## Data Flow

```
User invokes tp command
          |
        CLI Parser (clap) [cli.rs]
          |
    Command Router [main.rs]
          |
  6 Command Handlers:
  - cmd_init()      -> Initialize tree-pool.toml
  - cmd_get()       -> Acquire/reuse a tree, exec subshell
  - cmd_status()    -> Display pool status with process info
  - cmd_return()    -> Return tree to clean state
  - cmd_destroy()   -> Permanently delete tree(s)
  - cmd_update()    -> Self-update via cargo install
          |
State Management (state.rs)
Config Management (config.rs)
Git Operations (git.rs)
```

## Pool Directory Structure

```
~/.tree-pool/
  <repo-name>-<shortHash>/          pool_dir, identified by repo's origin URL hash
    .meta/
      tree-pool-state.json          persistent state: list of active trees
      tree-pool-state.lock          file lock for concurrent access
    1/                              first tree (numeric name)
      <repo-name>/                  actual clone directory
        .git/                       hardlinked objects with real remote
        <files...>
    2/                              second tree
      <repo-name>/
    N/                              nth tree
      <repo-name>/
```

The `.meta/` directory holds state files, keeping the pool root clean and supporting automatic migration from legacy flat structure.

## State Management

### Persistence and Locking (state.rs)

```rust
let _lock = state::State::lock(&pool_dir)?;    // Exclusive lock acquired
let mut st = state::State::load(&pool_dir)?;   // Load state from disk
// ... mutations ...
st.save(&pool_dir)?;                           // Write back to JSON
drop(_lock);                                   // Lock released on drop
```

State structure:

```rust
pub struct State {
    pub trees: Vec<TreeEntry>,
}

pub struct TreeEntry {
    pub name: String,              // Numeric ID: "1", "2", "3"
    pub path: PathBuf,             // Absolute path to clone
    pub created_at: DateTime<Utc>,
}
```

Self-healing: on `load()`, automatically removes entries whose paths no longer exist. Supports backward-compatible deserialization via serde alias (`worktrees` -> `trees`). Automatically migrates old state files from pool root to `.meta/`.

## Command Flows

### cmd_get() -- Acquire Tree

The most complex command:

1. Resolve repo root from current directory
2. Load config (repo-level or `~/.config/tree-pool/config.toml`)
3. Resolve pool_dir (using config.root + repo name + short hash of origin URL)
4. Fetch origin in source repo to refresh branch list
5. Resolve branch (explicit argument or interactive picker)
6. Acquire file lock on state
7. Load state
8. Try to reuse available tree (not in-use AND not dirty):
   - Fetch from `local` and `origin` remotes to refresh
   - Reset tracked files to clean state (untracked files like build artifacts are preserved), checkout requested branch
9. If no available tree and under max_trees: create new clone:
   - `git clone --local` (hardlinks .git/objects)
   - Rename `origin` remote to `local`
   - Add real `origin` remote if source has one
   - Checkout branch
10. Drop lock before spawning shell
11. `exec` into interactive subshell (or print path if non-TTY)

Lock ownership: held during state mutation and tree setup, dropped before spawning subshell. This allows concurrent `tp status`, `tp return`, etc.

### cmd_status() -- Display Pool

For each tree: detect processes with cwd inside tree path (via sysinfo), check dirty state, classify as "here" (cyan), "in-use" (red), "dirty" (yellow), or "available" (green). A tree can be "in-use" even if not dirty (a process is running in it).

### cmd_return() -- Reset Tree

Resolve tree path (from argument, `TREE_POOL_DIR` env var, or cwd). Validate no dirty state or unpushed branches (unless `--force`). Reset tracked files to detached state on default branch (untracked files like build artifacts are preserved). Tree stays in state for reuse.

### cmd_destroy() -- Delete Tree

Same safety checks as return (dirty, unpushed branches) unless `--force`. Deletes the numbered parent directory via `rm -rf`. Removes entry from state.

## Git Operations (git.rs)

### Layered Design

```
High-level:   clone_local, reset_tree, create_and_checkout_branch, unpushed_branches
Mid-level:    is_dirty, branch_exists, default_branch, branch_ref
Low-level:    run_git (stdout + error on non-zero), run_git_ok (bool)
```

All git interaction shells out to the `git` binary (no libgit2).

### Smart Branch Reference Selection (branch_ref)

When reusing a tree, decides whether to reset to local or remote ref:

1. Only local exists -> use local
2. Only remote exists -> use remote
3. Both exist: if local is ancestor of remote -> use remote (pull in latest); if remote is ancestor of local -> use local (keep local work); if diverged -> prefer remote

### Unpushed Branch Detection

For each local branch, checks ALL remotes to see if the branch exists and is not ahead. Handles multiple remotes (origin, local) and ensures the user is warned about all unpushed work.

## Configuration (config.rs)

Precedence (highest to lowest):

1. Repo-level: `<repo-root>/tree-pool.toml`
2. User-level: `~/.config/tree-pool/config.toml`
3. Defaults: `max_trees = 16`, `root = ""`

Root path resolution: empty string -> home directory, absolute path -> used as-is, relative path -> relative to repo root. Supports env var expansion via shellexpand.

Pool directory identification: `<root>/.tree-pool/<repo-name>-<shortHash>/` where shortHash = first 6 hex chars of SHA256(origin URL or repo path).

## Two-Remote Model

Every clone has exactly 2 remotes:

- **local**: points to the source repo (for fast fetches via hardlinks)
- **origin**: real upstream (GitHub, etc.), only if source has one

This allows `git fetch local` for fast refresh, `git push origin` for real upstream pushes, and prevents accidental pushes to the wrong remote.

## Notable Patterns

**Path canonicalization everywhere.** State lookups, path display, and gitignore management all canonicalize paths to handle macOS `/tmp` -> `/private/tmp` symlinks and different path representations.

**Conservative dirty detection.** `is_dirty` returns `true` on error (`unwrap_or(true)`), so trees are never reused when their state is uncertain.

**Process detection via sysinfo.** Enumerates all system processes and checks their cwd against the tree path using proper path component checking (not string prefix matching).

**Exec, not spawn.** `tp get` uses Unix `exec` to replace itself with the subshell rather than spawning a child process. This means the shell is the direct terminal process (Ctrl-D works), and no `tp` process sits idle waiting.

**TREE_POOL_DIR env var.** Set in the subshell so `tp return` (without arguments) knows which tree to return, even if the user has `cd`'d elsewhere.

**Idempotent operations.** Fetching, resetting, gitignore management, and state healing are all safe to repeat. This makes the tool resilient to interruption.
