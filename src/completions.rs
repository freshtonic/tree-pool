use std::ffi::OsStr;

use clap_complete::engine::CompletionCandidate;

use crate::{config, git, pool, state};

/// Complete tree paths for `tp return` and `tp destroy`.
/// Reads pool state to enumerate all known tree paths.
pub fn tree_path_candidates() -> Vec<CompletionCandidate> {
    let Some((_, pool_dir)) = resolve_pool_dir() else {
        return vec![];
    };

    let Ok(_lock) = state::State::lock(&pool_dir) else {
        return vec![];
    };
    let Ok(state) = state::State::load(&pool_dir) else {
        return vec![];
    };

    state
        .trees
        .iter()
        .map(|tree| {
            let path = tree.path.to_string_lossy().to_string();
            let help = format!("tree {}", tree.name);
            CompletionCandidate::new(path).help(Some(help.into()))
        })
        .collect()
}

/// Complete branch names for `tp get`.
/// Lists local and remote branches sorted by recent commit date.
pub fn branch_completer(current: &OsStr) -> Vec<CompletionCandidate> {
    let current = current.to_str().unwrap_or("");

    let Some((repo_root, _)) = resolve_pool_dir() else {
        return vec![];
    };

    let Ok(branches) = git::list_branches_by_date(&repo_root) else {
        return vec![];
    };

    branches
        .into_iter()
        .filter(|b| b.starts_with(current))
        .map(CompletionCandidate::new)
        .collect()
}

/// Resolve the repo root and pool directory from the current working directory.
/// Returns None if we can't determine them (e.g., not in a git repo).
fn resolve_pool_dir() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let cwd = std::env::current_dir().ok()?;
    let root = git::repo_root(&cwd).ok()?;
    let repo_root = std::path::PathBuf::from(&root);
    let config = config::Config::load(&repo_root).ok()?;
    let pool_dir = pool::resolve_pool_dir(&repo_root, &config).ok()?;
    Some((repo_root, pool_dir))
}
