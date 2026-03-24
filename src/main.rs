mod cli;
#[allow(dead_code)] // Functions will be used by future modules.
mod config;
#[allow(dead_code)] // Functions will be used by future modules.
mod git;
#[allow(dead_code)] // Functions will be used by future modules.
mod pool;
#[allow(dead_code)] // Functions will be used by future modules.
mod process;
#[allow(dead_code)] // Functions will be used by future modules.
mod prompt;
#[allow(dead_code)] // Functions will be used by future modules.
mod gitignore;
mod display;
#[allow(dead_code)] // Functions will be used by future modules.
mod shell;
#[allow(dead_code)] // Functions will be used by future modules.
mod state;

use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        None | Some(Command::Get) => cmd_get(),
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

fn cmd_get() -> anyhow::Result<()> {
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

    let _lock = state::State::lock(&pool_dir)?;
    let mut st = state::State::load(&pool_dir)?;

    // Try to find an available worktree (not in-use and not dirty)
    let available = st.worktrees.iter().find(|wt| {
        !process::is_in_use(&wt.path) && !git::is_dirty(&wt.path).unwrap_or(true)
    });

    let wt_path = if let Some(wt) = available {
        let wt_path = wt.path.clone();
        // Reset to latest default branch
        eprintln!("fetching origin...");
        git::fetch_origin(&repo_root_path)?;
        let branch = git::default_branch(&repo_root_path)?;
        let ref_name = git::branch_ref(&repo_root_path, &branch)?;
        git::reset_worktree(&wt_path, &ref_name)?;
        eprintln!("reusing worktree: {}", display::pretty_path(&wt_path));
        wt_path
    } else if st.worktrees.len() < config.max_trees {
        // Create a new worktree
        eprintln!("fetching origin...");
        git::fetch_origin(&repo_root_path)?;
        let branch = git::default_branch(&repo_root_path)?;
        let ref_name = git::branch_ref(&repo_root_path, &branch)?;

        let name = st.next_name();
        let wt_path = pool::worktree_path(&pool_dir, &name, &repo_name);

        // Create parent dir
        if let Some(parent) = wt_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        git::worktree_add(&repo_root_path, &wt_path, &ref_name)?;
        st.add(name, wt_path.clone());
        st.save(&pool_dir)?;
        eprintln!("created worktree: {}", display::pretty_path(&wt_path));
        wt_path
    } else {
        anyhow::bail!(
            "all {} worktrees are in use or dirty — run `tp status` to see details, \
             `tp return` to return a dirty worktree, or increase max_trees in tree-pool.toml",
            config.max_trees
        );
    };

    // Drop the lock before spawning the subshell
    drop(_lock);

    let exit_code = shell::spawn_subshell(&wt_path)?;

    // On exit, check if dirty and prompt
    if git::is_dirty(&wt_path).unwrap_or(false) {
        if prompt::confirm("worktree has uncommitted changes. return it anyway?", true)
            .unwrap_or(true)
        {
            let branch = git::default_branch(&repo_root_path)?;
            let ref_name = git::branch_ref(&repo_root_path, &branch)?;
            if let Err(e) = git::reset_worktree(&wt_path, &ref_name) {
                eprintln!("warning: failed to reset worktree: {e}");
            }
        }
    } else {
        // Clean exit — release the worktree
        let branch = git::default_branch(&repo_root_path)?;
        let ref_name = git::branch_ref(&repo_root_path, &branch)?;
        if let Err(e) = git::reset_worktree(&wt_path, &ref_name) {
            eprintln!("warning: failed to reset worktree: {e}");
        }
    }

    std::process::exit(exit_code);
}

fn cmd_status() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let root = git::repo_root(&cwd)?;
    let repo_root = Path::new(&root);
    let config = config::Config::load(repo_root)?;
    let pool_dir = pool::resolve_pool_dir(repo_root, &config)?;

    let _lock = state::State::lock(&pool_dir)?;
    let state = state::State::load(&pool_dir)?;

    if state.worktrees.is_empty() {
        eprintln!("no worktrees in pool");
        return Ok(());
    }

    use colored::Colorize;

    for wt in &state.worktrees {
        let procs = process::processes_in_dir(&wt.path);
        let dirty = git::is_dirty(&wt.path).unwrap_or(false);
        let current = display::is_current_dir(&wt.path);

        let (status_str, status_colored) = if current {
            ("here", "here".cyan().bold().to_string())
        } else if !procs.is_empty() {
            ("in-use", "in-use".red().to_string())
        } else if dirty {
            ("dirty", "dirty".yellow().to_string())
        } else {
            ("available", "available".green().to_string())
        };

        let _ = status_str; // suppress unused warning
        println!(
            "{:>4}  {:<11}  {}",
            wt.name,
            status_colored,
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

    // Resolve the worktree path
    let wt_path = if let Some(p) = path {
        PathBuf::from(p).canonicalize()?
    } else if let Ok(env_path) = std::env::var("TREE_POOL_DIR") {
        PathBuf::from(env_path).canonicalize()?
    } else {
        cwd.canonicalize()?
    };

    let _lock = state::State::lock(&pool_dir)?;
    let st = state::State::load(&pool_dir)?;

    // Validate this is a known worktree
    if st.find_by_path(&wt_path).is_none() {
        anyhow::bail!("{} is not a tree-pool worktree", wt_path.display());
    }

    // Check dirty
    if git::is_dirty(&wt_path)? && !force
        && !prompt::confirm("worktree has uncommitted changes. return it anyway?", true)?
    {
        return Ok(());
    }

    // Reset to clean state
    let branch = git::default_branch(&repo_root_path)?;
    let ref_name = git::branch_ref(&repo_root_path, &branch)?;
    git::reset_worktree(&wt_path, &ref_name)?;
    eprintln!("returned {}", display::pretty_path(&wt_path));
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
        if st.worktrees.is_empty() {
            eprintln!("no worktrees to destroy");
            return Ok(());
        }

        if !force
            && !prompt::confirm(
                &format!("destroy all {} worktrees?", st.worktrees.len()),
                false,
            )?
        {
            return Ok(());
        }

        let paths: Vec<_> = st.worktrees.iter().map(|wt| wt.path.clone()).collect();
        for wt_path in &paths {
            if !force && process::is_in_use(wt_path) {
                eprintln!(
                    "skipping {} (in use) — use --force to override",
                    display::pretty_path(wt_path)
                );
                continue;
            }
            destroy_worktree(&repo_root_path, &pool_dir, wt_path, &mut st)?;
        }
    } else {
        let path = path.context("path argument is required (or use --all)")?;
        let wt_path = PathBuf::from(&path).canonicalize()?;

        if st.find_by_path(&wt_path).is_none() {
            anyhow::bail!("{} is not a tree-pool worktree", wt_path.display());
        }

        if !force {
            if process::is_in_use(&wt_path) {
                anyhow::bail!(
                    "{} is in use — use --force to override",
                    display::pretty_path(&wt_path)
                );
            }

            if !prompt::confirm(
                &format!("destroy worktree {}?", display::pretty_path(&wt_path)),
                false,
            )? {
                return Ok(());
            }
        }

        destroy_worktree(&repo_root_path, &pool_dir, &wt_path, &mut st)?;
    }

    st.save(&pool_dir)?;
    Ok(())
}

fn destroy_worktree(
    repo_root: &Path,
    _pool_dir: &Path,
    wt_path: &Path,
    st: &mut state::State,
) -> anyhow::Result<()> {
    // Remove git worktree
    let _ = git::worktree_remove(repo_root, wt_path);

    // Remove the numbered parent directory (e.g., <poolDir>/1/)
    if let Some(parent) = wt_path.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }

    st.remove_by_path(wt_path);
    eprintln!("destroyed {}", display::pretty_path(wt_path));
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
