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
