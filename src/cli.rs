use clap::{Parser, Subcommand};
use clap_complete::engine::{ArgValueCandidates, ArgValueCompleter};

use crate::completions;

#[derive(Debug, Parser)]
#[command(
    name = "tp",
    version = concat!("tree-pool ", env!("CARGO_PKG_VERSION")),
    about = "Manage a pool of reusable local git clones"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create tree-pool.toml in the repo root
    Init,
    /// Show pool status
    Status,
    /// Acquire a tree from the pool
    Get {
        /// Branch to check out in the tree
        #[arg(add = ArgValueCompleter::new(completions::branch_completer))]
        branch: Option<String>,
    },
    /// Return a tree to the pool
    Return {
        /// Path to the tree to return
        #[arg(add = ArgValueCandidates::new(completions::tree_path_candidates))]
        path: Option<String>,
        /// Skip dirty-check prompt
        #[arg(long)]
        force: bool,
    },
    /// Remove a tree from the pool permanently
    Destroy {
        /// Path to the tree to destroy
        #[arg(add = ArgValueCandidates::new(completions::tree_path_candidates))]
        path: Option<String>,
        /// Force destroy even if dirty or has unpushed branches
        #[arg(long)]
        force: bool,
        /// Destroy all trees
        #[arg(long)]
        all: bool,
    },
    /// Update tree-pool via cargo install
    Update,
    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::aot::Shell,
    },
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
        assert!(matches!(cli.command, Some(Command::Get { .. })));
    }

    #[test]
    fn get_with_branch() {
        let cli = Cli::parse_from(["tp", "get", "feature/foo"]);
        match cli.command {
            Some(Command::Get { branch }) => {
                assert_eq!(branch.as_deref(), Some("feature/foo"));
            }
            _ => panic!("expected Get command"),
        }
    }

    #[test]
    fn get_without_branch() {
        let cli = Cli::parse_from(["tp", "get"]);
        match cli.command {
            Some(Command::Get { branch }) => {
                assert!(branch.is_none());
            }
            _ => panic!("expected Get command"),
        }
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

    #[test]
    fn completions_subcommand() {
        let cli = Cli::parse_from(["tp", "completions", "bash"]);
        assert!(matches!(cli.command, Some(Command::Completions { .. })));
    }
}
