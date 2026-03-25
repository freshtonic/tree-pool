use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

/// Spawn a subshell in the given tree directory.
/// Sets TREE_POOL_DIR in the environment.
/// Returns the shell's exit code.
pub fn spawn_subshell(tree_path: &Path) -> Result<i32> {
    let shell = resolve_shell();

    let mut child = Command::new(&shell)
        .current_dir(tree_path)
        .env("TREE_POOL_DIR", tree_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn shell: {shell}"))?;

    let status = child.wait().context("failed to wait for shell")?;
    Ok(status.code().unwrap_or(1))
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
