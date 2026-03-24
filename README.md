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
