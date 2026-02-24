use super::SelfImproveManager;
use super::coding_agent::{CodingAgentBackend, CodingAgentConfig};
use std::path::PathBuf;
use std::time::Duration;

/// Helper: create a CodingAgentConfig for testing.
fn test_agent_config(source_root: &std::path::Path) -> CodingAgentConfig {
    CodingAgentConfig {
        backend: CodingAgentBackend::ClaudeCode,
        binary_path: PathBuf::from("/usr/bin/claude"),
        source_root: source_root.to_path_buf(),
        timeout: Duration::from_secs(60),
        max_turns: None,
        model: None,
        api_key: None,
        api_key_env: "ANTHROPIC_API_KEY".to_string(),
    }
}

/// Helper: initialize a git repo in the given directory with a dummy initial commit.
fn init_git_repo(dir: &std::path::Path) {
    use std::process::Command;

    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .expect("git init failed");

    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .expect("git config user.name failed");

    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .expect("git config user.email failed");

    // Create an initial file and commit so HEAD exists.
    std::fs::write(dir.join("README.md"), "# Test\n").unwrap();

    Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .output()
        .expect("git add failed");

    Command::new("git")
        .args(["commit", "-m", "initial commit"])
        .current_dir(dir)
        .output()
        .expect("git commit failed");
}

#[test]
fn test_manager_initialization() {
    let source_tmp = tempfile::TempDir::new().unwrap();
    let ckpt_tmp = tempfile::TempDir::new().unwrap();

    let manager = SelfImproveManager::new(
        source_tmp.path().to_str().unwrap(),
        ckpt_tmp.path().to_str().unwrap(),
        5,
        test_agent_config(source_tmp.path()),
    );

    // Verify sub-managers are accessible.
    assert!(manager.checkpoint.list().is_empty());
    assert!(manager.active_session.is_none());
}

#[tokio::test]
async fn test_git_commit_flow() {
    let source_tmp = tempfile::TempDir::new().unwrap();
    let ckpt_tmp = tempfile::TempDir::new().unwrap();

    init_git_repo(source_tmp.path());

    let manager = SelfImproveManager::new(
        source_tmp.path().to_str().unwrap(),
        ckpt_tmp.path().to_str().unwrap(),
        5,
        test_agent_config(source_tmp.path()),
    );

    // Write a file directly (simulating what the coding agent would do).
    std::fs::write(source_tmp.path().join("hello.txt"), "Hello, world!\n").unwrap();

    // Stage and commit via git manager.
    let commit_result = manager.git.commit_all("add hello.txt").await;
    assert!(commit_result.is_ok());

    // Verify it was committed by checking the log.
    let log = manager.git.log(1).await.unwrap();
    assert!(log.contains("add hello.txt"));
}

#[test]
fn test_checkpoint_flow() {
    let source_tmp = tempfile::TempDir::new().unwrap();
    let ckpt_tmp = tempfile::TempDir::new().unwrap();

    let manager = SelfImproveManager::new(
        source_tmp.path().to_str().unwrap(),
        ckpt_tmp.path().to_str().unwrap(),
        5,
        test_agent_config(source_tmp.path()),
    );

    // Create a fake binary to checkpoint.
    let bin_path = source_tmp.path().join("fake-binary");
    std::fs::write(&bin_path, b"binary-content").unwrap();

    // Create a checkpoint.
    let checkpoint = manager
        .checkpoint
        .create("abcdef1234567890", &[bin_path])
        .unwrap();

    assert!(checkpoint.id.starts_with("ckpt-abcdef12"));
    assert_eq!(checkpoint.git_hash, "abcdef1234567890");
    assert_eq!(checkpoint.binaries.len(), 1);

    // Verify checkpoint directory exists.
    let ckpt_dir = ckpt_tmp.path().join(&checkpoint.id);
    assert!(ckpt_dir.is_dir());
    assert!(ckpt_dir.join("checkpoint.json").exists());
}

#[tokio::test]
async fn test_full_flow_commit_checkpoint() {
    let source_tmp = tempfile::TempDir::new().unwrap();
    let ckpt_tmp = tempfile::TempDir::new().unwrap();

    init_git_repo(source_tmp.path());

    let manager = SelfImproveManager::new(
        source_tmp.path().to_str().unwrap(),
        ckpt_tmp.path().to_str().unwrap(),
        5,
        test_agent_config(source_tmp.path()),
    );

    // Step 1: Write a file directly (simulating coding agent).
    std::fs::write(
        source_tmp.path().join("lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
    )
    .unwrap();

    // Step 2: Commit via git manager.
    manager.git.commit_all("add lib.rs").await.unwrap();

    // Step 3: Get the commit hash.
    let hash = manager.git.current_hash().await.unwrap();
    assert!(!hash.is_empty());

    // Step 4: Create a fake binary and checkpoint.
    let bin_path = source_tmp.path().join("target-binary");
    std::fs::write(&bin_path, b"compiled-output").unwrap();

    let checkpoint = manager
        .checkpoint
        .create(&hash, &[bin_path])
        .unwrap();

    // Step 5: Verify checkpoint was listed.
    let checkpoints = manager.checkpoint.list();
    assert_eq!(checkpoints.len(), 1);
    assert_eq!(checkpoints[0].id, checkpoint.id);

    // Step 6: Restore the checkpoint.
    let restored = manager.checkpoint.restore(&checkpoint.id).unwrap();
    assert_eq!(restored.git_hash, hash);
    assert_eq!(restored.binaries.len(), 1);
}
