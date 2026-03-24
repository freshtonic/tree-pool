use std::io::{self, BufRead, Write};

use anyhow::{Result, bail};

/// Prompt the user with a yes/no question. Returns true for yes, false for no.
/// `default_yes` controls what happens when the user presses Enter without typing.
pub fn confirm(message: &str, default_yes: bool) -> Result<bool> {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    eprint!("{message} {suffix} ");
    io::stderr().flush()?;

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input.is_empty() {
        return Ok(default_yes);
    }

    match input.as_str() {
        "y" | "yes" => Ok(true),
        "n" | "no" => Ok(false),
        _ => bail!("invalid input: {input}"),
    }
}

// No unit tests — interactive I/O. Tested manually / via integration tests.
