use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

/// Replace the current process with an interactive shell in the given tree directory.
/// Sets TREE_POOL_DIR in the environment.
/// This function does not return on success (the process is replaced).
pub fn exec_subshell(tree_path: &Path) -> Result<()> {
    use std::os::unix::process::CommandExt;

    let shell = resolve_shell();

    let err = Command::new(&shell)
        .current_dir(tree_path)
        .env("TREE_POOL_DIR", tree_path)
        .exec();

    Err(err).with_context(|| format!("failed to exec shell: {shell}"))
}

#[cfg(not(windows))]
fn resolve_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

#[cfg(windows)]
fn resolve_shell() -> String {
    std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
}

// No unit tests for this module — it spawns interactive shells.
// Tested via integration tests.
