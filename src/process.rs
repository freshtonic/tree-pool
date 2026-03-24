use std::path::Path;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

#[derive(Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
}

/// Find all processes whose current working directory is inside the given path.
/// Uses proper path component checking (not string prefix).
pub fn processes_in_dir(dir: &Path) -> Vec<ProcessInfo> {
    let dir = match dir.canonicalize() {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cwd(UpdateKind::Always),
    );

    let mut result = Vec::new();

    for (pid, process) in sys.processes() {
        let Some(cwd) = process.cwd() else {
            continue;
        };

        let cwd = match cwd.canonicalize() {
            Ok(c) => c,
            Err(_) => continue,
        };

        if cwd.starts_with(&dir) {
            result.push(ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
            });
        }
    }

    result
}

/// Check if any process is using the given directory.
pub fn is_in_use(dir: &Path) -> bool {
    !processes_in_dir(dir).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_dir_is_detected() {
        let cwd = std::env::current_dir().unwrap();
        let procs = processes_in_dir(&cwd);
        assert!(!procs.is_empty(), "expected at least this test process");
    }

    #[test]
    fn nonexistent_dir_returns_empty() {
        let procs = processes_in_dir(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(procs.is_empty());
    }
}
