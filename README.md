# tree-pool

A CLI tool (`tp`) for managing pools of reusable, pre-warmed local git clones for parallel AI coding agent workflows.

![tree-pool demo](demo.webp)

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

## Walkthrough

A quick tour using a fictional repo called `acme-api`.

**Initialize tree-pool in your repo:**

```bash
$ cd ~/projects/acme-api
$ tp init
created /Users/you/projects/acme-api/tree-pool.toml
```

**Get a tree to work on a new feature:**

```bash
$ tp get feature/billing
fetching origin...
created tree: ~/.tree-pool/acme-api-a1b2c3/1/acme-api
on branch: feature/billing
```

You're now in a subshell inside the clone. Do your work, commit, push, then exit:

```bash
$ git commit -am "feat: add billing endpoint"
$ git push origin feature/billing
$ exit
```

**Back in the source repo, get another tree for a hotfix:**

```bash
$ tp get fix/auth-timeout
fetching origin...
created tree: ~/.tree-pool/acme-api-a1b2c3/2/acme-api
on branch: fix/auth-timeout
```

Both trees exist simultaneously — each is an independent clone, so there's no branch conflict. Exit when done:

```bash
$ git commit -am "fix: increase auth token TTL"
$ git push origin fix/auth-timeout
$ exit
```

**Check pool status at any time:**

```bash
$ tp status
   1  available     feature/billing       ~/.tree-pool/acme-api-a1b2c3/1/acme-api
   2  available     fix/auth-timeout      ~/.tree-pool/acme-api-a1b2c3/2/acme-api
```

Both trees are available for reuse. The next `tp get` will reuse one instead of cloning.

**Return a tree explicitly (if it has unpushed work):**

```bash
$ tp return ~/.tree-pool/acme-api-a1b2c3/1/acme-api
returned ~/.tree-pool/acme-api-a1b2c3/1/acme-api
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

The command surface is roughly equivalent but while `treehouse` manages pools of git worktrees, `tree-pool` manages pools of local git clones.
