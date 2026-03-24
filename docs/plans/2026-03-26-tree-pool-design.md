# tree-pool Design

Rust port of [treehouse](https://github.com/kunchenguid/treehouse) — a CLI tool (`tp`) that manages a pool of reusable, pre-warmed git worktrees for parallel AI coding agent workflows.

## CLI Structure

| Command | Description | Flags |
|---|---|---|
| `tp get` (default) | Acquire a worktree, open subshell | — |
| `tp status` | Show pool status with color-coded states | — |
| `tp return [path]` | Return a worktree to the pool | `--force` (skip dirty prompt) |
| `tp destroy [path]` | Remove worktree permanently | `--force`, `--all` |
| `tp init` | Create `tree-pool.toml` in repo root | — |
| `tp update` | Run `cargo install tree-pool` | — |
| `tp --version` | Print version | — |

Running `tp` with no subcommand runs `get`. Errors go to stderr without printing usage help.

## Dependencies

| Crate | Purpose |
|---|---|
| `clap` (derive) | CLI framework |
| `serde` + `toml` | Config parsing |
| `serde_json` | State file |
| `colored` | Terminal colors |
| `sysinfo` | Cross-platform process enumeration |
| `fs2` | Cross-platform file locking |
| `sha2` | Pool directory hashing |

## Worktree Pool Management

### Pool Directory

Located at `~/.tree-pool/<repoName>-<shortHash>/` by default. The hash is the first 6 hex chars of SHA-256 of the origin remote URL (or absolute repo path if no remote). If config `root` is set, the pool lives at `<root>/.tree-pool/<repoName>-<shortHash>/` instead.

Physical layout per worktree: `<poolDir>/<number>/<repoName>/`

### Acquire (`tp get`)

1. Find repo root via `git rev-parse --show-toplevel`
2. Load config (repo-level then user-level)
3. Resolve pool directory
4. Ensure `.gitignore` covers pool dir (if pool is inside repo)
5. Scan pool for a worktree that is both not in-use and not dirty
6. If found: `git fetch origin`, then reset to latest default branch (detached HEAD)
7. If not found and pool not full: `git worktree add --detach`, increment sequential name
8. If pool full and all in-use/dirty: error with helpful message
9. Spawn subshell with `$TREE_POOL_DIR` set to worktree path
10. On subshell exit: check dirty, prompt user if dirty, release if clean

### Release

Run `git checkout --detach <ref>` then `git clean -fd` to reset the worktree to a clean detached state.

### Default Branch Detection (in order)

1. `git symbolic-ref refs/remotes/origin/HEAD`
2. `git symbolic-ref HEAD` of the main repo
3. `git config init.defaultBranch`
4. Error with helpful message

### Detached HEAD Ref Selection

When resetting a worktree, compare local `refs/heads/<branch>` vs `origin/<branch>` using `git merge-base --is-ancestor`. If diverged, prefer `origin/<branch>`.

## State Management

### State File

Location: `<poolDir>/tree-pool-state.json`

```json
{
  "worktrees": [
    {
      "name": "1",
      "path": "/absolute/path/to/worktree",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

Missing state file = empty state (not an error). Corrupt JSON = fatal error.

### File Locking

Lock file: `<poolDir>/tree-pool-state.lock`

All state reads/writes happen inside a lock. Using the `fs2` crate for cross-platform locking (`flock` on Unix, `LockFileEx` on Windows). Lock file is created if it doesn't exist.

### Self-Healing

On every state read, entries whose paths no longer exist on disk are silently removed and the state file is rewritten.

## Process Detection

Uses `sysinfo` crate to enumerate all running processes. A worktree is "in-use" if any process has a cwd equal to or inside the worktree path. Uses proper path prefix checking (not string prefix). Per-process errors are silently skipped.

Returns a list of `(pid, name)` for matching processes — used by `tp status` to show what's using each worktree.

### Dirty Detection

Runs `git status --porcelain` in the worktree directory. Any output = dirty.

## Configuration

TOML format. First found wins:
1. `<repo_root>/tree-pool.toml`
2. `~/.config/tree-pool/config.toml`

| Field | Type | Default | Description |
|---|---|---|---|
| `max_trees` | integer | 16 | Maximum worktrees in the pool |
| `root` | string | `""` (= `$HOME`) | Base directory for pool. Relative to repo root, absolute, or with env var expansion. |

No config file = defaults silently. TOML parse error = fatal.

## Subshell

- Unix: `$SHELL`, fallback `/bin/sh`
- Windows: `%COMSPEC%`, fallback `cmd.exe`
- Inherits parent environment, adds `TREE_POOL_DIR=<worktree_path>`
- stdin/stdout/stderr connected directly to terminal
- Captures exit code

## Commands

### `tp status`

One line per worktree: name (padded), status (color-coded), path (with `~` for home). In-use worktrees show process names and PIDs on an indented second line.

Colors: green=available, red=in-use, yellow=dirty, cyan+bold=current.

### `tp return [path]`

Path resolution: explicit arg > `$TREE_POOL_DIR` > cwd. Validates the path is a known worktree. Checks dirty, prompts for confirmation unless `--force`.

### `tp destroy [path]`

With `--all`: destroys every worktree. Without: path argument required. Without `--force`: prompts for confirmation (default=no). Runs `git worktree remove --force` and removes the numbered parent directory.

### `tp init`

Creates `tree-pool.toml` at repo root with defaults. Errors if file already exists.

### `tp update`

Runs `cargo install tree-pool`. Prints success or failure.

### Auto .gitignore

When pool dir is inside a repo, `tp get` ensures the pool path is in `.gitignore`. Idempotent — checks for existing entry before appending.

## Error Handling

- Git errors: capture stderr and wrap into error message
- State I/O and lock errors: fatal
- Non-fatal warnings (gitignore failures, release failures on subshell exit): print to stderr
- Dirty-worktree prompt decline: leave worktree dirty, not an error
- Process CWD read errors: silently skipped per-process

## Platform Support

Primary: macOS + Linux. Windows: best-effort, second priority.

Platform-specific behavior:
- File locking: `fs2` handles cross-platform
- Subshell: `$SHELL` / `%COMSPEC%`
- Process detection: `sysinfo` handles cross-platform

## Differences from treehouse

| Aspect | Treehouse | tree-pool |
|---|---|---|
| Language | Go | Rust |
| CLI command | `treehouse` | `tp` |
| File/dir naming | `treehouse-*` | `tree-pool-*` |
| Distribution | GitHub releases | `cargo install tree-pool` |
| Self-update | Download binary, verify checksum, atomic replace | `cargo install tree-pool` |
| Background update checker | Detached child process + 24h cache | Removed |
| Windows support | First-class | Best-effort, second priority |
