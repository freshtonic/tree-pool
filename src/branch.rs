use std::path::Path;

use anyhow::{Result, bail};
use colored::Colorize;
use dialoguer::{Input, Select, theme::ColorfulTheme};

use crate::git;

const CREATE_NEW_LABEL: &str = "[ Create new branch ]";

/// Prompt the user to select a branch interactively or create a new one.
/// Returns the branch name and whether it's a new branch.
pub fn select_branch(repo_root: &Path) -> Result<(String, bool)> {
    let all_branches = git::list_branches_by_date(repo_root)?;
    let checked_out = git::checked_out_branches(repo_root)?;

    // Build display items: first is "create new", rest are branches
    let mut items: Vec<String> = vec![CREATE_NEW_LABEL.to_string()];
    let mut selectable: Vec<bool> = vec![true];

    for branch in &all_branches {
        let is_checked_out = checked_out.contains(branch);
        if is_checked_out {
            items.push(format!("{}", branch.dimmed()));
        } else {
            items.push(branch.clone());
        }
        selectable.push(!is_checked_out);
    }

    if items.len() == 1 {
        // No branches to show, go straight to create
        return prompt_new_branch(&all_branches);
    }

    loop {
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("select a branch")
            .items(&items)
            .default(0)
            .interact()?;

        if !selectable[selection] {
            eprintln!("that branch is already checked out in another worktree");
            continue;
        }

        if selection == 0 {
            return prompt_new_branch(&all_branches);
        }

        return Ok((all_branches[selection - 1].clone(), false));
    }
}

/// Validate a branch name provided via CLI argument.
/// Returns error if the branch is already checked out.
pub fn validate_branch(repo_root: &Path, branch: &str) -> Result<bool> {
    let checked_out = git::checked_out_branches(repo_root)?;
    if checked_out.contains(branch) {
        bail!("branch '{branch}' is already checked out in another worktree");
    }

    // Check if branch exists locally or on remote
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
