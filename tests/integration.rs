use std::process::Command;

fn tp() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tp"))
}

#[test]
fn version_flag() {
    let output = tp().arg("--version").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tree-pool"));
}

#[test]
fn init_creates_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    // Set up a git repo
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();
    std::fs::write(path.join("file.txt"), "hello").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(path)
        .output()
        .unwrap();

    let output = tp().arg("init").current_dir(path).output().unwrap();
    assert!(
        output.status.success(),
        "tp init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(path.join("tree-pool.toml").exists());

    // Second init should fail
    let output = tp().arg("init").current_dir(path).output().unwrap();
    assert!(!output.status.success());
}

#[test]
fn get_creates_clone_with_remotes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    // Set up a git repo with an initial commit
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();
    std::fs::write(path.join("file.txt"), "hello").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(path)
        .output()
        .unwrap();

    // Get the default branch name
    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(path)
        .output()
        .unwrap();
    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    // Run tp get with the branch in non-interactive mode (pipe stdin)
    let output = Command::new(env!("CARGO_BIN_EXE_tp"))
        .args(["get", &branch])
        .current_dir(path)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tp get failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // stdout should contain the clone path
    let clone_path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let clone_path = std::path::Path::new(&clone_path_str);
    assert!(
        clone_path.join(".git").exists(),
        "clone should have .git directory"
    );

    // Clone should have a "local" remote pointing to source
    let remotes = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(clone_path)
        .output()
        .unwrap();
    let remotes_str = String::from_utf8_lossy(&remotes.stdout);
    assert!(
        remotes_str.contains("local"),
        "clone should have 'local' remote"
    );

    // Clean up: destroy the tree
    let _ = Command::new(env!("CARGO_BIN_EXE_tp"))
        .args(["destroy", "--force", &clone_path_str])
        .current_dir(path)
        .output();
}

#[test]
fn status_in_empty_pool() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();
    std::fs::write(path.join("file.txt"), "hello").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(path)
        .output()
        .unwrap();

    let output = tp().arg("status").current_dir(path).output().unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no trees"));
}
