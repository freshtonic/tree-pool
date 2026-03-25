mod branch;
mod cli;
mod config;
mod display;
mod git;
mod gitignore;
mod pool;
mod process;
mod prompt;
mod shell;
mod state;

use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Command::Get { branch }) => cmd_get(branch),
        None => cmd_get(None),
        Some(Command::Status) => cmd_status(),
        Some(Command::Return { path, force }) => cmd_return(path, force),
        Some(Command::Destroy { path, force, all }) => cmd_destroy(path, force, all),
        Some(Command::Init) => cmd_init(),
        Some(Command::Update) => cmd_update(),
    };

    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn cmd_get(branch: Option<String>) -> anyhow::Result<()> {
    use std::io::IsTerminal;

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

    // Try to find an available tree (not in-use and not dirty)
    let available = st
        .trees
        .iter()
        .find(|t| !process::is_in_use(&t.path) && !git::is_dirty(&t.path).unwrap_or(true));

    let tree_path = if let Some(tree) = available {
        let tree_path = tree.path.clone();

        // Fetch from local remote to pick up latest source repo state
        if let Err(e) = git::fetch_remote(&tree_path, "local") {
            eprintln!("warning: failed to fetch local remote: {e}");
        }

        // Fetch from origin if it exists
        if git::has_remote(&tree_path, "origin")?
            && let Err(e) = git::fetch_remote(&tree_path, "origin")
        {
            eprintln!("warning: failed to fetch origin: {e}");
        }

        // Reset and checkout the requested branch
        let default = git::default_branch(&repo_root_path)?;
        let ref_name = git::branch_ref(&tree_path, &default)?;
        git::reset_tree(&tree_path, &ref_name)?;

        if is_new {
            git::create_and_checkout_branch(&tree_path, &selected_branch)?;
        } else {
            git::checkout_branch(&tree_path, &selected_branch)?;
        }

        eprintln!("reusing tree: {}", display::pretty_path(&tree_path));
        tree_path
    } else if st.trees.len() < config.max_trees {
        let name = st.next_name();
        let tp = pool::tree_path(&pool_dir, &name, &repo_name);

        if let Some(parent) = tp.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Clone locally — objects are hardlinked
        git::clone_local(&repo_root_path, &tp)?;

        // Set up remotes: rename origin→local, add real origin if source has one
        git::rename_remote(&tp, "origin", "local")?;
        if let Some(url) = git::remote_url(&repo_root_path)? {
            git::add_remote(&tp, "origin", &url)?;
            if let Err(e) = git::fetch_remote(&tp, "origin") {
                eprintln!("warning: failed to fetch origin: {e}");
            }
        }

        // Checkout the requested branch
        if is_new {
            git::create_and_checkout_branch(&tp, &selected_branch)?;
        } else {
            git::checkout_branch(&tp, &selected_branch)?;
        }

        let tp = tp.canonicalize().unwrap_or(tp);
        st.add(name, tp.clone());
        st.save(&pool_dir)?;
        eprintln!("created tree: {}", display::pretty_path(&tp));
        tp
    } else {
        anyhow::bail!(
            "all {} trees are in use or dirty — run `tp status` to see details, \
             `tp return` to free a tree, or increase max_trees in tree-pool.toml",
            config.max_trees
        );
    };

    // Drop the lock before spawning the subshell
    drop(_lock);

    eprintln!("on branch: {selected_branch}");

    // If interactive TTY, open subshell. Otherwise, print path.
    if std::io::stdin().is_terminal() {
        let exit_code = shell::spawn_subshell(&tree_path)?;
        std::process::exit(exit_code);
    } else {
        println!("{}", tree_path.display());
    }

    Ok(())
}

fn cmd_status() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let root = git::repo_root(&cwd)?;
    let repo_root = Path::new(&root);
    let config = config::Config::load(repo_root)?;
    let pool_dir = pool::resolve_pool_dir(repo_root, &config)?;

    let _lock = state::State::lock(&pool_dir)?;
    let state = state::State::load(&pool_dir)?;

    if state.trees.is_empty() {
        eprintln!("no worktrees in pool");
        return Ok(());
    }

    use colored::Colorize;

    for wt in &state.trees {
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

    Ok(())
}

fn cmd_return(path: Option<String>, force: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let root = git::repo_root(&cwd)?;
    let repo_root_path = PathBuf::from(&root);
    let config = config::Config::load(&repo_root_path)?;
    let pool_dir = pool::resolve_pool_dir(&repo_root_path, &config)?;

    // Resolve the tree path
    let tree_path = if let Some(p) = path {
        PathBuf::from(p).canonicalize()?
    } else if let Ok(env_path) = std::env::var("TREE_POOL_DIR") {
        PathBuf::from(env_path).canonicalize()?
    } else {
        cwd.canonicalize()?
    };

    let _lock = state::State::lock(&pool_dir)?;
    let st = state::State::load(&pool_dir)?;

    if st.find_by_path(&tree_path).is_none() {
        anyhow::bail!("{} is not a tree-pool tree", tree_path.display());
    }

    if !force {
        // Check dirty
        if git::is_dirty(&tree_path)? {
            anyhow::bail!("tree has uncommitted changes — commit or discard them, or use --force");
        }

        // Check for unpushed branches
        let unpushed = git::unpushed_branches(&tree_path)?;
        if !unpushed.is_empty() {
            anyhow::bail!(
                "tree has unpushed branches: {} — push them or use --force",
                unpushed.join(", ")
            );
        }
    }

    // Reset to clean state
    let default = git::default_branch(&tree_path)?;
    git::reset_tree(&tree_path, &format!("refs/heads/{default}"))?;
    eprintln!("returned {}", display::pretty_path(&tree_path));
    Ok(())
}

fn cmd_destroy(path: Option<String>, force: bool, all: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let root = git::repo_root(&cwd)?;
    let repo_root_path = PathBuf::from(&root);
    let config = config::Config::load(&repo_root_path)?;
    let pool_dir = pool::resolve_pool_dir(&repo_root_path, &config)?;

    let _lock = state::State::lock(&pool_dir)?;
    let mut st = state::State::load(&pool_dir)?;

    if all {
        if st.trees.is_empty() {
            eprintln!("no trees to destroy");
            return Ok(());
        }

        if !force && !prompt::confirm(&format!("destroy all {} trees?", st.trees.len()), false)? {
            return Ok(());
        }

        let paths: Vec<_> = st.trees.iter().map(|t| t.path.clone()).collect();
        for tree_path in &paths {
            if !force && process::is_in_use(tree_path) {
                eprintln!(
                    "skipping {} (in use) — use --force to override",
                    display::pretty_path(tree_path)
                );
                continue;
            }
            if let Err(e) = destroy_tree(tree_path, &mut st, force) {
                eprintln!("warning: {e}");
                continue;
            }
        }
    } else {
        let path = path.context("path argument is required (or use --all)")?;
        let tree_path = PathBuf::from(&path).canonicalize()?;

        if st.find_by_path(&tree_path).is_none() {
            anyhow::bail!("{} is not a tree-pool tree", tree_path.display());
        }

        if !force {
            if process::is_in_use(&tree_path) {
                anyhow::bail!(
                    "{} is in use — use --force to override",
                    display::pretty_path(&tree_path)
                );
            }

            if !prompt::confirm(
                &format!("destroy tree {}?", display::pretty_path(&tree_path)),
                false,
            )? {
                return Ok(());
            }
        }

        destroy_tree(&tree_path, &mut st, force)?;
    }

    st.save(&pool_dir)?;
    Ok(())
}

fn destroy_tree(tree_path: &Path, st: &mut state::State, force: bool) -> anyhow::Result<()> {
    if !force {
        if git::is_dirty(tree_path)? {
            anyhow::bail!(
                "{} has uncommitted changes — use --force to override",
                display::pretty_path(tree_path)
            );
        }
        let unpushed = git::unpushed_branches(tree_path)?;
        if !unpushed.is_empty() {
            anyhow::bail!(
                "{} has unpushed branches: {} — use --force to override",
                display::pretty_path(tree_path),
                unpushed.join(", ")
            );
        }
    }

    // Remove the numbered parent directory (e.g., <poolDir>/1/)
    if let Some(parent) = tree_path.parent()
        && let Err(e) = std::fs::remove_dir_all(parent)
    {
        eprintln!("warning: failed to remove directory: {e}");
    }

    st.remove_by_path(tree_path);
    eprintln!("destroyed {}", display::pretty_path(tree_path));
    Ok(())
}

fn cmd_init() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let root = git::repo_root(&cwd)?;
    let config_path = Path::new(&root).join("tree-pool.toml");

    if config_path.exists() {
        anyhow::bail!("tree-pool.toml already exists at {}", config_path.display());
    }

    let content = config::Config::default_toml();
    std::fs::write(&config_path, content)?;
    eprintln!("created {}", config_path.display());
    Ok(())
}

fn cmd_update() -> anyhow::Result<()> {
    eprintln!("updating tree-pool...");
    let status = std::process::Command::new("cargo")
        .args(["install", "tree-pool"])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        anyhow::bail!("cargo install tree-pool failed");
    }

    Ok(())
}
