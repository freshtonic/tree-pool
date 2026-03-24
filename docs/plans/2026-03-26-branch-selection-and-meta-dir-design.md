# Branch Selection & .meta Directory Design

## Overview

Two changes to tree-pool:

1. `tp get` accepts a branch name (or prompts with an interactive selector)
2. Pool metadata files move from the pool root into a `.meta/` subdirectory

## Feature 1: Branch Selection

### CLI

`tp get [branch]` — optional positional argument.

### When branch is provided

Use it directly. Error if the branch is already checked out in another worktree or the parent repo.

### When branch is omitted

Show an interactive `dialoguer` selector:

- First item: `[ Create new branch ]` — prompts for a branch name, creates from the default branch's HEAD
- Remaining items: local + remote branches sorted by most recent commit date, deduplicated (if both `main` and `origin/main` exist, show once as `main`)
- Branches already checked out in a worktree or the parent repo are shown dimmed and unselectable

### Worktree lifecycle

**On `tp get`:**
- Reusing an available worktree: checkout the selected branch (not detached HEAD reset)
- Creating a new worktree with a new branch: `git worktree add <path> -b <branch>`
- Creating a new worktree with an existing branch: `git worktree add <path> <branch>`

**On return (exit subshell or `tp return`):**
- Reset the worktree back to detached HEAD on the default branch — frees the branch name for future use

**On `tp status`:**
- Show the current branch name alongside each worktree entry

## Feature 2: .meta Directory

### Layout change

Before:
```
~/.tree-pool/myrepo-abc123/
  tree-pool-state.json
  tree-pool-state.lock
  1/myrepo/
  2/myrepo/
```

After:
```
~/.tree-pool/myrepo-abc123/
  .meta/
    tree-pool-state.json
    tree-pool-state.lock
  1/myrepo/
  2/myrepo/
```

### Migration

On first run, if old files exist at pool root and `.meta/` doesn't exist, move them automatically. No user action required.

## Files changed

- `cli.rs` — Add optional `branch: Option<String>` to `Get`
- `git.rs` — New: `list_branches_by_date()`, `checked_out_branches()`, `worktree_add_branch()`
- `main.rs` — Rework `cmd_get()` for branch selection; update return logic
- `state.rs` — Update paths to use `.meta/`; add migration
- `display.rs` — Show branch name in `tp status`

## New dependency

`dialoguer` for interactive branch selection.
