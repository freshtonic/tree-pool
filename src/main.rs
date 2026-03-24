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
    todo!()
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
