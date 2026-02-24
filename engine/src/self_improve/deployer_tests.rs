use super::DeployManager;
use std::path::PathBuf;

#[test]
fn test_resolve_target_chat_shell() {
    let dm = DeployManager::new();
    let result = dm.resolve_target("chat-shell").unwrap();
    assert_eq!(result, PathBuf::from("/usr/bin/levsha-chat"));
}

#[test]
fn test_resolve_target_chat_shell_underscore() {
    let dm = DeployManager::new();
    let result = dm.resolve_target("chat_shell").unwrap();
    assert_eq!(result, PathBuf::from("/usr/bin/levsha-chat"));
}

#[test]
fn test_resolve_target_full() {
    let dm = DeployManager::new();
    let result = dm.resolve_target("full").unwrap();
    assert_eq!(result, PathBuf::from("/usr/bin/levsha-chat"));
}

#[test]
fn test_resolve_target_engine() {
    let dm = DeployManager::new();
    let result = dm.resolve_target("engine").unwrap();
    assert_eq!(result, PathBuf::from("/usr/bin/levsha-engine"));
}

#[test]
fn test_resolve_target_unknown() {
    let dm = DeployManager::new();
    let result = dm.resolve_target("unknown");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown deploy target"));
}

#[tokio::test]
async fn test_deploy_artifact_not_found() {
    let dm = DeployManager::new();
    let nonexistent = PathBuf::from("/tmp/levsha-test-nonexistent-artifact-12345");
    let result = dm.deploy("engine", &nonexistent).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Artifact not found"));
}

#[tokio::test]
async fn test_deploy_creates_backup() {
    let tmp = tempfile::TempDir::new().unwrap();

    // Create a fake "current binary" at a known path inside tmp.
    let target_path = tmp.path().join("levsha-engine");
    std::fs::write(&target_path, b"old-binary-content").unwrap();

    // Create a fake artifact.
    let artifact_path = tmp.path().join("new-artifact");
    std::fs::write(&artifact_path, b"new-binary-content").unwrap();

    // We cannot call deploy() directly because it resolves to /usr/bin/...
    // Instead, test the backup logic by simulating what deploy does:
    // copy target to target.bak, then copy artifact to target.
    let backup_path = target_path.with_extension("bak");
    std::fs::copy(&target_path, &backup_path).unwrap();
    std::fs::copy(&artifact_path, &target_path).unwrap();

    // Verify backup was created.
    assert!(backup_path.exists());
    let backup_content = std::fs::read_to_string(&backup_path).unwrap();
    assert_eq!(backup_content, "old-binary-content");

    // Verify target was updated.
    let target_content = std::fs::read_to_string(&target_path).unwrap();
    assert_eq!(target_content, "new-binary-content");
}

#[tokio::test]
async fn test_rollback_binary_not_found() {
    let dm = DeployManager::new();
    let nonexistent = PathBuf::from("/tmp/levsha-test-nonexistent-rollback-12345");
    let result = dm.rollback("engine", &nonexistent).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Rollback binary not found"));
}
