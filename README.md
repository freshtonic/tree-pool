# tree-pool

A CLI tool (`tp`) for managing pools of reusable, pre-warmed local git clones for parallel AI coding agent workflows.

## Install

```
cargo install tree-pool
```

## Usage

```bash
tp init               # Create tree-pool.toml in repo root
tp status             # Show pool status (includes branch names)
tp                    # Acquire a tree (prompts for branch)
tp get                # Same as above
tp get <branch>       # Acquire a tree on a specific branch
tp return             # Return a tree to the pool
tp destroy            # Remove a tree permanently
tp update             # Update tree-pool via cargo install
```

## Configuration

Create `tree-pool.toml` in your repo root (or `~/.config/tree-pool/config.toml` for global config):

```toml
max_trees = 16
# root = ""  # Base directory for the pool (default: home directory)
```

## How It Works

Each tree in the pool is a local git clone created with `git clone --local`, which hardlinks `.git/objects` for near-zero disk overhead. Each clone gets two remotes:

- **origin** — the real upstream remote (e.g. GitHub), if the source repo has one
- **local** — points to the source repo on disk

Because clones are fully independent repositories, multiple trees can have the same branch checked out simultaneously, and filesystem sandboxing is straightforward.

## Credits

This project is a Rust port of [treehouse](https://github.com/kunchenguid/treehouse) by [kunchenguid](https://github.com/kunchenguid).
