use super::GitManager;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Create a temporary git repo with an initial commit.
fn setup_git_repo() -> (TempDir, GitManager) {
    let dir = TempDir::new().expect("failed to create temp dir");
    let path = dir.path();

    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .expect("git init failed");

    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .expect("git config user.name failed");

    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .expect("git config user.email failed");

    fs::write(path.join("initial.txt"), "initial content").unwrap();

    Command::new("git")
        .args(["add", "-A"])
        .current_dir(path)
        .output()
        .expect("git add failed");

    Command::new("git")
        .args(["commit", "-m", "initial commit"])
        .current_dir(path)
        .output()
        .expect("git commit failed");

    let mgr = GitManager::new(path.to_path_buf());
    (dir, mgr)
}

#[tokio::test]
async fn test_commit_all_creates_commit() {
    let (dir, mgr) = setup_git_repo();
    fs::write(dir.path().join("new.txt"), "new content").unwrap();

    let result = mgr.commit_all("test commit").await;
    assert!(result.is_ok());

    let log = mgr.log(5).await.unwrap();
    assert!(log.contains("test commit"));
}

#[tokio::test]
async fn test_diff_working_shows_changes() {
    let (dir, mgr) = setup_git_repo();

    // Modify a tracked file
    fs::write(dir.path().join("initial.txt"), "modified content").unwrap();

    let diff = mgr.diff_working().await.unwrap();
    assert!(diff.contains("modified content"));
}

#[tokio::test]
async fn test_diff_staged_shows_staged() {
    let (dir, mgr) = setup_git_repo();

    fs::write(dir.path().join("initial.txt"), "staged change").unwrap();
    Command::new("git")
        .args(["add", "initial.txt"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let diff = mgr.diff_staged().await.unwrap();
    assert!(diff.contains("staged change"));
}

#[tokio::test]
async fn test_log_returns_entries() {
    let (_dir, mgr) = setup_git_repo();

    let log = mgr.log(5).await.unwrap();
    assert!(log.contains("initial commit"));
}

#[tokio::test]
async fn test_current_hash_is_valid() {
    let (_dir, mgr) = setup_git_repo();

    let hash = mgr.current_hash().await.unwrap();
    // A full SHA-1 hash is 40 hex characters
    assert_eq!(hash.len(), 40);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn test_tag_creates_tag() {
    let (dir, mgr) = setup_git_repo();

    mgr.tag("v0.1.0").await.unwrap();

    let output = Command::new("git")
        .args(["tag", "-l"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let tags = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(tags.contains("v0.1.0"));
}

#[tokio::test]
async fn test_diff_file_list() {
    let (dir, mgr) = setup_git_repo();

    fs::write(dir.path().join("initial.txt"), "changed").unwrap();

    let files = mgr.diff_file_list().await.unwrap();
    assert!(files.contains(&"initial.txt".to_string()));
}

#[tokio::test]
async fn test_show_file_at_head() {
    let (_dir, mgr) = setup_git_repo();

    let content = mgr.show_file("initial.txt").await.unwrap();
    assert_eq!(content, "initial content");
}

#[tokio::test]
async fn test_show_file_nonexistent() {
    let (_dir, mgr) = setup_git_repo();

    let content = mgr.show_file("does_not_exist.txt").await.unwrap();
    assert!(content.is_empty());
}
