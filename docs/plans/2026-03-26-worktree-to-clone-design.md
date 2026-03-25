# Switch from Git Worktrees to Local Clones

## Motivation

Git worktrees share "current branch" state with the parent repo and siblings, which causes problems:
- Two worktrees cannot check out the same branch simultaneously
- Committing in a worktree requires the source repo to be writable (making filesystem sandboxing impossible for AI agents)
- Branch state is globally shared, leading to surprising interactions

Local clones (`git clone --local`) solve all of these. Objects are hardlinked so disk overhead is near-zero, and each clone is a fully independent repository.

## Core Model

Each "tree" in the pool is a full local git clone. Each clone gets two remotes:
- **`origin`** -- the real upstream remote (e.g. GitHub), copied from the source repo's origin URL. Only set up if the source repo has an origin remote.
- **`local`** -- points to the source repo's local path on disk. Always exists.

The pool is create-on-demand: `tp get` creates a new clone if no idle tree is available, up to `max_trees`. No pre-warming or background processes.

Operations that touch `origin` (fetch on `get`, unpushed-branch check on `return`) gracefully skip when origin is absent or unreachable.

## Lifecycle: `tp get`

1. Resolve the source repo root and pool directory (same as today)
2. Acquire the state lock
3. Look for an idle tree in the pool
4. **If idle tree found:**
   - Reset it: `git checkout <branch> && git reset --hard && git clean -fd`
   - Fetch from `local` remote to pick up latest local state
   - Fetch from `origin` remote (skip if no origin or if it fails)
5. **If no idle tree:**
   - `git clone --local <source-repo> <tree-path>`
   - If source repo has an `origin` remote: rename the clone's `origin` to `local`, then add `origin` pointing to the real upstream URL
   - If source repo has no `origin`: rename the clone's `origin` (pointing to local path) to `local`, no `origin` is added
   - Check out the requested branch
6. Mark the tree as in-use in state, save
7. If interactive TTY, open subshell. Otherwise, print the path.

Branch selection works as today: optional argument, or interactive picker if omitted in a TTY. The picker no longer filters out already-checked-out branches (clones don't share branch state).

## Lifecycle: `tp return`

1. Resolve which tree is being returned (from argument or current directory)
2. Acquire the state lock
3. **Safety checks** (skip all if `--force`):
   - Check if working tree or index is dirty
   - Check for local branches with commits not present on `local` or `origin` remotes. For each branch, verify it exists on at least one remote and is not ahead. If `origin` doesn't exist, only check against `local`.
   - If any check fails, display an error describing what's unpushed/dirty and exit without returning
4. Reset the tree: detach HEAD, hard reset, clean
5. Mark the tree as idle in state, save

## Lifecycle: `tp destroy`

1. Resolve which tree(s) to destroy (from argument, current directory, or `--all`)
2. Acquire the state lock
3. **Safety checks** (skip if `--force`), same as `return`
4. `rm -rf` the tree directory
5. Remove the entry from state, save

With `--force --all`, removes everything unconditionally.

## Naming Changes

All terminology changes from "worktree" to "tree":

- `WorktreeEntry` -> `TreeEntry`, `state.worktrees` -> `state.trees`
- CLI help text updated throughout
- `worktree_path()` -> `tree_path()`
- `reset_worktree()` -> `reset_tree()`

State file name (`tree-pool-state.json`) stays the same. Config (`tree-pool.toml`) unchanged -- `max_trees` already correct.

## What Gets Removed

- `git::worktree_add_new_branch()` and `git::worktree_add_existing_branch()` -- replaced by `git clone --local`
- `git::worktree_remove()` -- replaced by `rm -rf`
- `git::checked_out_branches()` -- no longer needed (clones don't share branch state)
- Branch exclusion logic in the interactive picker

## What Gets Added

- `git::clone_local()` -- `git clone --local <source> <dest>`
- `git::rename_remote()` -- `git remote rename <old> <new>`
- `git::add_remote()` -- `git remote add <name> <url>`
- `git::has_unpushed_branches()` -- check each local branch against `local` and `origin` remotes
- `git::fetch_remote()` -- fetch a specific remote by name (generalizes `fetch_origin`)
