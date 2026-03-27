# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this project is

tree-pool (`tp`) is a Rust CLI that manages pools of reusable local git clones for parallel AI coding agent workflows. Each clone uses `git clone --local` (hardlinked objects, near-zero disk overhead) and gets two remotes: `origin` (real upstream) and `local` (source repo on disk).

## Commands

```bash
cargo build                          # build
cargo test                           # all tests (unit + integration)
cargo test --lib                     # unit tests only
cargo test --test integration        # integration tests only
cargo test <test_name>               # single test by name
cargo clippy -- -D warnings          # lint (zero warnings policy)
cargo fmt --check                    # format check
cargo run -- <args>                  # run tp with arguments
```

All three checks (fmt, clippy, test) must pass before committing. CI enforces this.

## Architecture

Main document: [ARCHITECTURE.md](ARCHITECTURE.md).

Commands are routed in `main.rs` via clap (`cli.rs`). Each `cmd_*` function orchestrates config, state, and git operations.

**State management** (`state.rs`): Pool state lives in `<pool_dir>/.meta/tree-pool-state.json` with file-based locking (`fs2`). State self-heals on load by pruning entries whose paths no longer exist. The lock is held only during state mutation — dropped before spawning subshells.

**Git operations** (`git.rs`): All git interaction shells out to the `git` binary (no libgit2). Functions are layered: `run_git`/`run_git_ok` at the bottom, then predicates (`is_dirty`, `branch_exists`), then high-level operations (`clone_local`, `reset_tree`, `unpushed_branches`).

**Tree lifecycle**: `cmd_get` either reuses an idle tree (not in-use, not dirty) by resetting it, or creates a new clone. It then `exec`s (not spawns) a shell — `tp` replaces itself with the subshell process. `cmd_return` validates no dirty state or unpushed branches (unless `--force`), then resets the tree for reuse. `cmd_destroy` does the same checks then `rm -rf`s the tree.

**Two-remote model**: Clones always have a `local` remote (source repo path) and optionally an `origin` remote (real upstream URL copied from source). Operations gracefully handle missing origin.

## Conventions

- Conventional Commits (`feat:`, `fix:`, `refactor:`, etc.)
- Run `cargo test --all-features`, `cargo clippy --all-features` and `cargo fmt`, address any problens before committing
— release-plz uses these for automated versioning
- Paths are canonicalized in state lookups to handle macOS `/tmp` → `/private/tmp` symlinks
- `anyhow::Result` with `.context()` throughout — no panics in command code
- Unit tests are colocated in each module; integration tests in `tests/integration.rs`
- Test repos use `tempfile::tempdir()` for isolation

## Documentation

Consolidate project knowledge by keeping CLAUDE.md (this doc), ARCHITECTURE.md, README.md & DEVELOPMENT.md up to date whenever a change is noteworthy enough to warrant a documentation revision.

## Demo recording

The animated demo in the README is generated with [vhs](https://github.com/charmbracelet/vhs):

```bash
./demo/record.sh
```

Uses fake `tp`/`git` commands (see `demo/setup.sh`) — no real git operations, runs in a temp dir.
