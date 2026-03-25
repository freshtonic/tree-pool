use std::path::Path;

use anyhow::{Result, bail};
use dialoguer::{Input, Select, theme::ColorfulTheme};

use crate::git;

const CREATE_NEW_LABEL: &str = "[ Create new branch ]";

/// Prompt the user to select a branch interactively or create a new one.
/// Returns the branch name and whether it's a new branch.
pub fn select_branch(repo_root: &Path) -> Result<(String, bool)> {
    let all_branches = git::list_branches_by_date(repo_root)?;

    let mut items: Vec<String> = vec![CREATE_NEW_LABEL.to_string()];
    for branch in &all_branches {
        items.push(branch.clone());
    }

    if items.len() == 1 {
        return prompt_new_branch(&all_branches);
    }

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("select a branch")
        .items(&items)
        .default(0)
        .interact()?;

    if selection == 0 {
        return prompt_new_branch(&all_branches);
    }

    Ok((all_branches[selection - 1].clone(), false))
}

/// Validate a branch name provided via CLI argument.
/// Returns whether the branch exists locally or on a remote.
pub fn validate_branch(repo_root: &Path, branch: &str) -> Result<bool> {
    let exists = git::branch_exists(repo_root, branch)?;
    Ok(exists)
}

fn prompt_new_branch(existing: &[String]) -> Result<(String, bool)> {
    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("new branch name")
        .interact_text()?;

    let name = name.trim().to_string();
    if name.is_empty() {
        bail!("branch name cannot be empty");
    }

    if existing.contains(&name) {
        bail!("branch '{name}' already exists — select it from the list instead");
    }

    Ok((name, true))
}
