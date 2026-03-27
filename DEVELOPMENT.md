# Development

## Prerequisites

- [Rust](https://rustup.rs/) (stable, edition 2024)
- [vhs](https://github.com/charmbracelet/vhs) (for recording the demo animation)

## Build

```bash
cargo build
cargo build --release
```

The binary is `tp`, located at `target/debug/tp` or `target/release/tp`.

## Tests

```bash
cargo test
```

Unit tests live alongside the source in each module (`src/*.rs`). Integration tests are in `tests/integration.rs`.

## Linting

All of these must pass before committing:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

CI runs these on every push to `main` and on pull requests.

## Commit conventions

This project uses [Conventional Commits](https://www.conventionalcommits.org/). The commit type determines how [release-plz](https://release-plz.dev/) bumps the version:

| Type | Version bump | Example |
|------|-------------|---------|
| `fix:` | patch | `fix: handle missing origin remote` |
| `feat:` | minor | `feat: add branch selection` |
| `feat!:` / `BREAKING CHANGE:` | major | `feat!: switch from worktrees to clones` |

Other types (`docs:`, `refactor:`, `test:`, `ci:`, `chore:`, `build:`, `style:`) don't trigger a release.

## Releases

Releases are automated via [release-plz](https://release-plz.dev/). On every push to `main`, it:

1. Creates or updates a release PR with a version bump and changelog
2. When that PR is merged, publishes to crates.io and creates a GitHub release

No manual version bumping or tagging is needed.

## Recording the demo

The animated demo in the README is generated with [vhs](https://github.com/charmbracelet/vhs):

```bash
./demo/record.sh
```

This produces `demo.webp` in the repo root. The recording runs in a temp directory and uses fake `tp`/`git` commands (defined in `demo/setup.sh`) so no files leak into the repo and no real git operations occur.

## Project layout

```
src/
  main.rs       CLI entry point and command implementations
  cli.rs        clap argument definitions
  git.rs        git command wrappers
  config.rs     tree-pool.toml parsing
  state.rs      pool state persistence (JSON + file locking)
  pool.rs       pool directory resolution
  branch.rs     interactive branch selection
  shell.rs      subshell spawning
  process.rs    process detection (is a tree in use?)
  display.rs    path formatting helpers
  prompt.rs     yes/no prompts
  gitignore.rs  .gitignore auto-management
tests/
  integration.rs
demo/
  demo.tape     vhs tape file
  setup.sh      fake environment for demo recording
  record.sh     one-command demo regeneration
```
