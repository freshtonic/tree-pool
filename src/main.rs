mod cli;
#[allow(dead_code)] // Functions will be used by future modules.
mod config;
#[allow(dead_code)] // Functions will be used by future modules.
mod git;
#[allow(dead_code)] // Functions will be used by future modules.
mod state;

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
    todo!()
}

fn cmd_update() -> anyhow::Result<()> {
    todo!()
}
