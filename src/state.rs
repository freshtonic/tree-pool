use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeEntry {
    pub name: String,
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub worktrees: Vec<WorktreeEntry>,
}

/// Guard that holds the lock file open. Lock is released when dropped.
pub struct StateLock {
    _file: File,
}

impl State {
    /// Acquire an exclusive lock on the state lock file.
    pub fn lock(pool_dir: &Path) -> Result<StateLock> {
        fs::create_dir_all(pool_dir)
            .with_context(|| format!("failed to create pool dir {}", pool_dir.display()))?;

        let lock_path = pool_dir.join("tree-pool-state.lock");
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .with_context(|| format!("failed to open lock file {}", lock_path.display()))?;

        file.lock_exclusive()
            .context("failed to acquire state lock")?;

        Ok(StateLock { _file: file })
    }

    /// Read state from disk. Returns empty state if file doesn't exist.
    /// Heals stale entries (paths that no longer exist on disk).
    pub fn load(pool_dir: &Path) -> Result<Self> {
        let state_path = pool_dir.join("tree-pool-state.json");

        if !state_path.exists() {
            return Ok(State::default());
        }

        let contents = fs::read_to_string(&state_path)
            .with_context(|| format!("failed to read {}", state_path.display()))?;

        let mut state: State = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse {}", state_path.display()))?;

        // Self-healing: remove entries whose paths no longer exist
        let before = state.worktrees.len();
        state.worktrees.retain(|wt| wt.path.exists());
        if state.worktrees.len() != before {
            state.save(pool_dir)?;
        }

        Ok(state)
    }

    /// Write state to disk.
    pub fn save(&self, pool_dir: &Path) -> Result<()> {
        fs::create_dir_all(pool_dir)
            .with_context(|| format!("failed to create pool dir {}", pool_dir.display()))?;

        let state_path = pool_dir.join("tree-pool-state.json");
        let contents = serde_json::to_string_pretty(self).context("failed to serialize state")?;

        fs::write(&state_path, contents)
            .with_context(|| format!("failed to write {}", state_path.display()))?;

        Ok(())
    }

    /// Find the next sequential worktree name (max existing + 1).
    pub fn next_name(&self) -> String {
        let max = self
            .worktrees
            .iter()
            .filter_map(|wt| wt.name.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        (max + 1).to_string()
    }

    /// Find a worktree entry by its absolute path.
    pub fn find_by_path(&self, path: &Path) -> Option<&WorktreeEntry> {
        self.worktrees.iter().find(|wt| wt.path == path)
    }

    /// Add a new worktree entry.
    pub fn add(&mut self, name: String, path: PathBuf) {
        self.worktrees.push(WorktreeEntry {
            name,
            path,
            created_at: Utc::now(),
        });
    }

    /// Remove a worktree entry by path.
    pub fn remove_by_path(&mut self, path: &Path) {
        self.worktrees.retain(|wt| wt.path != path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state_by_default() {
        let state = State::default();
        assert!(state.worktrees.is_empty());
    }

    #[test]
    fn next_name_empty_state() {
        let state = State::default();
        assert_eq!(state.next_name(), "1");
    }

    #[test]
    fn next_name_with_entries() {
        let mut state = State::default();
        state.add("1".to_string(), PathBuf::from("/a"));
        state.add("3".to_string(), PathBuf::from("/b"));
        assert_eq!(state.next_name(), "4");
    }

    #[test]
    fn find_by_path() {
        let mut state = State::default();
        state.add("1".to_string(), PathBuf::from("/a/b/c"));
        assert!(state.find_by_path(Path::new("/a/b/c")).is_some());
        assert!(state.find_by_path(Path::new("/x/y/z")).is_none());
    }

    #[test]
    fn remove_by_path() {
        let mut state = State::default();
        state.add("1".to_string(), PathBuf::from("/a"));
        state.add("2".to_string(), PathBuf::from("/b"));
        state.remove_by_path(Path::new("/a"));
        assert_eq!(state.worktrees.len(), 1);
        assert_eq!(state.worktrees[0].name, "2");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = State::default();
        state.add("1".to_string(), dir.path().to_path_buf()); // Use existing path so healing doesn't remove it
        state.save(dir.path()).unwrap();

        let loaded = State::load(dir.path()).unwrap();
        assert_eq!(loaded.worktrees.len(), 1);
        assert_eq!(loaded.worktrees[0].name, "1");
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = State::load(dir.path()).unwrap();
        assert!(state.worktrees.is_empty());
    }

    #[test]
    fn load_heals_stale_entries() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = State::default();
        state.add("1".to_string(), PathBuf::from("/nonexistent/path"));
        state.add("2".to_string(), dir.path().to_path_buf());
        // Write directly to avoid healing on save
        let state_path = dir.path().join("tree-pool-state.json");
        let contents = serde_json::to_string_pretty(&state).unwrap();
        fs::write(&state_path, contents).unwrap();

        let loaded = State::load(dir.path()).unwrap();
        assert_eq!(loaded.worktrees.len(), 1);
        assert_eq!(loaded.worktrees[0].name, "2");
    }

    #[test]
    fn lock_and_unlock() {
        let dir = tempfile::tempdir().unwrap();
        let _lock = State::lock(dir.path()).unwrap();
        // Lock is released when _lock is dropped
    }
}
