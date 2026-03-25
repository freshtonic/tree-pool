use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const DEFAULT_MAX_TREES: usize = 16;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_max_trees")]
    pub max_trees: usize,
    #[serde(default)]
    pub root: String,
}

fn default_max_trees() -> usize {
    DEFAULT_MAX_TREES
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_trees: DEFAULT_MAX_TREES,
            root: String::new(),
        }
    }
}

impl Config {
    /// Load config from repo-level or user-level file. Returns defaults if neither exists.
    pub fn load(repo_root: &Path) -> Result<Self> {
        // 1. Repo-level: <repo_root>/tree-pool.toml
        let repo_config = repo_root.join("tree-pool.toml");
        if repo_config.exists() {
            let contents = std::fs::read_to_string(&repo_config)
                .with_context(|| format!("failed to read {}", repo_config.display()))?;
            let config: Config = toml::from_str(&contents)
                .with_context(|| format!("failed to parse {}", repo_config.display()))?;
            return Ok(config);
        }

        // 2. User-level: ~/.config/tree-pool/config.toml
        if let Some(config_dir) = dirs::config_dir() {
            let user_config = config_dir.join("tree-pool").join("config.toml");
            if user_config.exists() {
                let contents = std::fs::read_to_string(&user_config)
                    .with_context(|| format!("failed to read {}", user_config.display()))?;
                let config: Config = toml::from_str(&contents)
                    .with_context(|| format!("failed to parse {}", user_config.display()))?;
                return Ok(config);
            }
        }

        // No config found -- use defaults
        Ok(Config::default())
    }

    /// Resolve the `root` config field to an absolute path.
    /// Empty string = home dir. Relative = relative to repo root. Supports env var expansion.
    pub fn resolve_root(&self, repo_root: &Path) -> Result<PathBuf> {
        if self.root.is_empty() {
            return dirs::home_dir().context("could not determine home directory");
        }

        let expanded = shellexpand::env(&self.root)
            .with_context(|| format!("failed to expand env vars in root: {}", self.root))?
            .to_string();

        let path = Path::new(&expanded);
        if path.is_absolute() {
            Ok(path.to_path_buf())
        } else {
            Ok(repo_root.join(path))
        }
    }

    /// Generate the default TOML content for `tp init`.
    pub fn default_toml() -> String {
        let config = Config::default();
        let mut content = toml::to_string_pretty(&config).unwrap_or_default();
        content.push_str("\n# root = \"\"  # Base directory for the tree pool.\n");
        content.push_str("# Relative paths are relative to the repo root.\n");
        content.push_str("# Supports environment variables, e.g. \"$HOME/worktrees\".\n");
        content.push_str("# Default: home directory (~/.tree-pool/)\n");
        content
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let config = Config::default();
        assert_eq!(config.max_trees, 16);
        assert_eq!(config.root, "");
    }

    #[test]
    fn parse_full_config() {
        let toml_str = r#"
            max_trees = 8
            root = "/tmp/worktrees"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.max_trees, 8);
        assert_eq!(config.root, "/tmp/worktrees");
    }

    #[test]
    fn parse_partial_config_uses_defaults() {
        let toml_str = r#"
            max_trees = 4
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.max_trees, 4);
        assert_eq!(config.root, "");
    }

    #[test]
    fn parse_empty_config_uses_defaults() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.max_trees, 16);
        assert_eq!(config.root, "");
    }

    #[test]
    fn resolve_root_empty_returns_home() {
        let config = Config::default();
        let root = config.resolve_root(Path::new("/repo")).unwrap();
        assert_eq!(root, dirs::home_dir().unwrap());
    }

    #[test]
    fn resolve_root_absolute() {
        let config = Config {
            max_trees: 16,
            root: "/tmp/custom".to_string(),
        };
        let root = config.resolve_root(Path::new("/repo")).unwrap();
        assert_eq!(root, Path::new("/tmp/custom"));
    }

    #[test]
    fn resolve_root_relative() {
        let config = Config {
            max_trees: 16,
            root: "worktrees".to_string(),
        };
        let root = config.resolve_root(Path::new("/repo")).unwrap();
        assert_eq!(root, Path::new("/repo/worktrees"));
    }

    #[test]
    fn load_returns_defaults_when_no_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert_eq!(config.max_trees, 16);
    }

    #[test]
    fn load_reads_repo_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("tree-pool.toml"), "max_trees = 3\n").unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert_eq!(config.max_trees, 3);
    }

    #[test]
    fn default_toml_is_valid() {
        let content = Config::default_toml();
        // The non-comment portion should parse
        let config: Config = toml::from_str(&content).unwrap();
        assert_eq!(config.max_trees, 16);
    }
}
