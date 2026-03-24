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

use std::path::Path;

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
    todo!()
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

fn cmd_return(_path: Option<String>, _force: bool) -> anyhow::Result<()> {
    todo!()
}

fn cmd_destroy(_path: Option<String>, _force: bool, _all: bool) -> anyhow::Result<()> {
    todo!()
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
