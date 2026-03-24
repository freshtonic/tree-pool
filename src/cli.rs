use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "tp",
    version,
    about = "Manage a pool of reusable git worktrees"
)]
#[command(disable_help_subcommand = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Acquire a worktree from the pool and open a subshell
    Get,
    /// Show pool status
    Status,
    /// Return a worktree to the pool
    Return {
        /// Path to the worktree to return
        path: Option<String>,
        /// Skip dirty-check prompt
        #[arg(long)]
        force: bool,
    },
    /// Remove a worktree from the pool permanently
    Destroy {
        /// Path to the worktree to destroy
        path: Option<String>,
        /// Force destroy even if in-use
        #[arg(long)]
        force: bool,
        /// Destroy all worktrees
        #[arg(long)]
        all: bool,
    },
    /// Create tree-pool.toml in the repo root
    Init,
    /// Update tree-pool via cargo install
    Update,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn no_subcommand_is_none() {
        let cli = Cli::parse_from(["tp"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn get_subcommand() {
        let cli = Cli::parse_from(["tp", "get"]);
        assert!(matches!(cli.command, Some(Command::Get)));
    }

    #[test]
    fn status_subcommand() {
        let cli = Cli::parse_from(["tp", "status"]);
        assert!(matches!(cli.command, Some(Command::Status)));
    }

    #[test]
    fn return_with_force() {
        let cli = Cli::parse_from(["tp", "return", "--force", "/some/path"]);
        match cli.command {
            Some(Command::Return { path, force }) => {
                assert_eq!(path.as_deref(), Some("/some/path"));
                assert!(force);
            }
            _ => panic!("expected Return command"),
        }
    }

    #[test]
    fn destroy_all() {
        let cli = Cli::parse_from(["tp", "destroy", "--all"]);
        match cli.command {
            Some(Command::Destroy { all, .. }) => assert!(all),
            _ => panic!("expected Destroy command"),
        }
    }

    #[test]
    fn version_flag() {
        let result = Cli::try_parse_from(["tp", "--version"]);
        assert!(result.is_err()); // clap exits on --version
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }
}
