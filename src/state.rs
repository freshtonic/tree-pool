use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    pub name: String,
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(alias = "worktrees")]
    pub trees: Vec<TreeEntry>,
}

/// Guard that holds the lock file open. Lock is released when dropped.
pub struct StateLock {
    _file: File,
}

impl State {
    fn meta_dir(pool_dir: &Path) -> PathBuf {
        pool_dir.join(".meta")
    }

    /// Acquire an exclusive lock on the state lock file.
    pub fn lock(pool_dir: &Path) -> Result<StateLock> {
        let meta = Self::meta_dir(pool_dir);
        fs::create_dir_all(&meta)
            .with_context(|| format!("failed to create meta dir {}", meta.display()))?;

        let lock_path = meta.join("tree-pool-state.lock");
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
    /// Automatically migrates state from the old location (pool root) to `.meta/`.
    pub fn load(pool_dir: &Path) -> Result<Self> {
        let meta = Self::meta_dir(pool_dir);
        Self::migrate_to_meta(pool_dir, &meta)?;

        let state_path = meta.join("tree-pool-state.json");

        if !state_path.exists() {
            return Ok(State::default());
        }

        let contents = fs::read_to_string(&state_path)
            .with_context(|| format!("failed to read {}", state_path.display()))?;

        let mut state: State = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse {}", state_path.display()))?;

        // Self-healing: remove entries whose paths no longer exist
        let before = state.trees.len();
        state.trees.retain(|wt| wt.path.exists());
        if state.trees.len() != before {
            state.save(pool_dir)?;
        }

        Ok(state)
    }

    /// Write state to disk.
    pub fn save(&self, pool_dir: &Path) -> Result<()> {
        let meta = Self::meta_dir(pool_dir);
        fs::create_dir_all(&meta)
            .with_context(|| format!("failed to create meta dir {}", meta.display()))?;

        let state_path = meta.join("tree-pool-state.json");
        let contents = serde_json::to_string_pretty(self).context("failed to serialize state")?;

        fs::write(&state_path, contents)
            .with_context(|| format!("failed to write {}", state_path.display()))?;

        Ok(())
    }

    /// Migrate state files from the old location (pool root) to `.meta/`.
    fn migrate_to_meta(pool_dir: &Path, meta: &Path) -> Result<()> {
        let old_state = pool_dir.join("tree-pool-state.json");
        let old_lock = pool_dir.join("tree-pool-state.lock");

        if !old_state.exists() && !old_lock.exists() {
            return Ok(());
        }
        if meta.join("tree-pool-state.json").exists() {
            let _ = fs::remove_file(&old_state);
            let _ = fs::remove_file(&old_lock);
            return Ok(());
        }

        fs::create_dir_all(meta)
            .with_context(|| format!("failed to create meta dir {}", meta.display()))?;

        if old_state.exists() {
            fs::rename(&old_state, meta.join("tree-pool-state.json"))
                .context("failed to migrate state file to .meta/")?;
        }
        if old_lock.exists() {
            let _ = fs::remove_file(&old_lock);
        }

        Ok(())
    }

    /// Find the next sequential tree name (max existing + 1).
    pub fn next_name(&self) -> String {
        let max = self
            .trees
            .iter()
            .filter_map(|wt| wt.name.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        (max + 1).to_string()
    }

    /// Find a tree entry by its absolute path.
    /// Canonicalizes both sides to handle symlinks and path normalization.
    pub fn find_by_path(&self, path: &Path) -> Option<&TreeEntry> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.trees.iter().find(|wt| {
            let wt_canonical = wt.path.canonicalize().unwrap_or_else(|_| wt.path.clone());
            wt_canonical == canonical
        })
    }

    /// Add a new tree entry.
    pub fn add(&mut self, name: String, path: PathBuf) {
        self.trees.push(TreeEntry {
            name,
            path,
            created_at: Utc::now(),
        });
    }

    /// Remove a tree entry by path.
    /// Canonicalizes both sides to handle symlinks and path normalization.
    pub fn remove_by_path(&mut self, path: &Path) {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.trees.retain(|wt| {
            let wt_canonical = wt.path.canonicalize().unwrap_or_else(|_| wt.path.clone());
            wt_canonical != canonical
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state_by_default() {
        let state = State::default();
        assert!(state.trees.is_empty());
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
        assert_eq!(state.trees.len(), 1);
        assert_eq!(state.trees[0].name, "2");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = State::default();
        state.add("1".to_string(), dir.path().to_path_buf()); // Use existing path so healing doesn't remove it
        state.save(dir.path()).unwrap();

        let loaded = State::load(dir.path()).unwrap();
        assert_eq!(loaded.trees.len(), 1);
        assert_eq!(loaded.trees[0].name, "1");
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = State::load(dir.path()).unwrap();
        assert!(state.trees.is_empty());
    }

    #[test]
    fn load_heals_stale_entries() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = State::default();
        state.add("1".to_string(), PathBuf::from("/nonexistent/path"));
        state.add("2".to_string(), dir.path().to_path_buf());
        // Write directly to .meta/ to avoid healing on save
        let meta = dir.path().join(".meta");
        fs::create_dir_all(&meta).unwrap();
        let state_path = meta.join("tree-pool-state.json");
        let contents = serde_json::to_string_pretty(&state).unwrap();
        fs::write(&state_path, contents).unwrap();

        let loaded = State::load(dir.path()).unwrap();
        assert_eq!(loaded.trees.len(), 1);
        assert_eq!(loaded.trees[0].name, "2");
    }

    #[test]
    fn lock_and_unlock() {
        let dir = tempfile::tempdir().unwrap();
        let _lock = State::lock(dir.path()).unwrap();
        // Lock is released when _lock is dropped
    }

    #[test]
    fn find_by_path_canonicalizes() {
        let dir = tempfile::tempdir().unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        let mut state = State::default();
        // Store the non-canonical path (e.g., /tmp/... on macOS which is a symlink to /private/tmp/...)
        state.add("1".to_string(), dir.path().to_path_buf());
        // Look up by canonical path
        assert!(state.find_by_path(&canonical).is_some());
    }

    #[test]
    fn remove_by_path_canonicalizes() {
        let dir = tempfile::tempdir().unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        let mut state = State::default();
        state.add("1".to_string(), dir.path().to_path_buf());
        // Remove using canonical path
        state.remove_by_path(&canonical);
        assert!(state.trees.is_empty());
    }

    #[test]
    fn lock_creates_meta_dir() {
        let dir = tempfile::tempdir().unwrap();
        let _lock = State::lock(dir.path()).unwrap();
        assert!(dir.path().join(".meta").exists());
        assert!(
            dir.path()
                .join(".meta")
                .join("tree-pool-state.lock")
                .exists()
        );
    }

    #[test]
    fn save_writes_to_meta_dir() {
        let dir = tempfile::tempdir().unwrap();
        let state = State::default();
        state.save(dir.path()).unwrap();
        assert!(
            dir.path()
                .join(".meta")
                .join("tree-pool-state.json")
                .exists()
        );
    }

    #[test]
    fn deserializes_worktrees_key_via_alias() {
        // Existing state files on disk use "worktrees" as the JSON key.
        // The serde alias ensures they still load correctly after the rename to "trees".
        let json = r#"{"worktrees":[{"name":"1","path":"/tmp/test","created_at":"2025-01-01T00:00:00Z"}]}"#;
        let state: State = serde_json::from_str(json).unwrap();
        assert_eq!(state.trees.len(), 1);
        assert_eq!(state.trees[0].name, "1");
    }

    #[test]
    fn serializes_as_trees_key() {
        let mut state = State::default();
        state.add("1".to_string(), PathBuf::from("/tmp/test"));
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"trees\""));
        assert!(!json.contains("\"worktrees\""));
    }

    #[test]
    fn migrate_moves_old_files_to_meta() {
        let dir = tempfile::tempdir().unwrap();
        let pool = dir.path();

        fs::write(pool.join("tree-pool-state.json"), r#"{"worktrees":[]}"#).unwrap();
        fs::write(pool.join("tree-pool-state.lock"), "").unwrap();

        let state = State::load(pool).unwrap();
        assert!(state.trees.is_empty());

        assert!(!pool.join("tree-pool-state.json").exists());
        assert!(!pool.join("tree-pool-state.lock").exists());

        assert!(pool.join(".meta").join("tree-pool-state.json").exists());
    }
}
