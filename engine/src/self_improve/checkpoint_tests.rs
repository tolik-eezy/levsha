use super::*;
use std::fs;
use std::path::PathBuf;

fn make_manager(dir: &std::path::Path, max: usize) -> CheckpointManager {
    CheckpointManager::new(dir.to_path_buf(), max)
}

/// Helper: write a fake binary file and return its path.
fn write_fake_binary(dir: &std::path::Path, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    path
}

/// Helper: create a checkpoint.json directly for testing list/restore.
fn write_checkpoint_json(
    checkpoint_dir: &std::path::Path,
    id: &str,
    git_hash: &str,
    timestamp: i64,
) {
    let ckpt_dir = checkpoint_dir.join(id);
    fs::create_dir_all(&ckpt_dir).unwrap();
    let ckpt = Checkpoint {
        id: id.to_string(),
        git_hash: git_hash.to_string(),
        timestamp,
        binaries: Vec::new(),
    };
    let json = serde_json::to_string_pretty(&ckpt).unwrap();
    fs::write(ckpt_dir.join("checkpoint.json"), json).unwrap();
}

#[test]
fn test_create_checkpoint() {
    let tmp = tempfile::tempdir().unwrap();
    let mgr = make_manager(tmp.path(), 5);

    let result = mgr.create("abcdef1234567890", &[]);
    assert!(result.is_ok());

    let ckpt = result.unwrap();
    assert!(ckpt.id.starts_with("ckpt-abcdef12-"));
    assert_eq!(ckpt.git_hash, "abcdef1234567890");
    assert!(ckpt.binaries.is_empty());

    // Verify metadata file exists.
    let meta_path = tmp.path().join(&ckpt.id).join("checkpoint.json");
    assert!(meta_path.exists());
}

#[test]
fn test_create_checkpoint_copies_binaries() {
    let tmp = tempfile::tempdir().unwrap();
    let bin_dir = tmp.path().join("bins");
    fs::create_dir_all(&bin_dir).unwrap();

    let bin1 = write_fake_binary(&bin_dir, "levsha-chat", b"binary-chat-content");
    let bin2 = write_fake_binary(&bin_dir, "levsha-engine", b"binary-engine-content");

    let ckpt_dir = tmp.path().join("checkpoints");
    let mgr = make_manager(&ckpt_dir, 5);

    let ckpt = mgr.create("abc12345deadbeef", &[bin1, bin2]).unwrap();
    assert_eq!(ckpt.binaries.len(), 2);

    // Verify binary content was copied.
    for saved in &ckpt.binaries {
        assert!(saved.exists());
    }
    let copied_chat = ckpt_dir.join(&ckpt.id).join("levsha-chat");
    assert_eq!(fs::read(&copied_chat).unwrap(), b"binary-chat-content");
}

#[test]
fn test_create_checkpoint_id_format() {
    let tmp = tempfile::tempdir().unwrap();
    let mgr = make_manager(tmp.path(), 5);

    let ckpt = mgr.create("1234567890abcdef", &[]).unwrap();
    // ID format: ckpt-<first 8 chars of hash>-<timestamp>
    assert!(ckpt.id.starts_with("ckpt-12345678-"));
    let parts: Vec<&str> = ckpt.id.splitn(3, '-').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "ckpt");
    assert_eq!(parts[1], "12345678");
    // Third part should be a valid timestamp.
    assert!(parts[2].parse::<i64>().is_ok());
}

#[test]
fn test_create_checkpoint_skips_missing_binaries() {
    let tmp = tempfile::tempdir().unwrap();
    let mgr = make_manager(tmp.path(), 5);

    let missing = PathBuf::from("/nonexistent/levsha-chat");
    let ckpt = mgr.create("aaaa1111bbbb2222", &[missing]).unwrap();
    assert!(ckpt.binaries.is_empty());
}

#[test]
fn test_list_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let mgr = make_manager(tmp.path(), 5);
    let list = mgr.list();
    assert!(list.is_empty());
}

#[test]
fn test_list_empty_nonexistent_dir() {
    let mgr = make_manager(&PathBuf::from("/tmp/nonexistent_ckpt_dir_xyz"), 5);
    let list = mgr.list();
    assert!(list.is_empty());
}

#[test]
fn test_list_returns_newest_first() {
    let tmp = tempfile::tempdir().unwrap();
    write_checkpoint_json(tmp.path(), "ckpt-old", "aaa", 1000);
    write_checkpoint_json(tmp.path(), "ckpt-new", "bbb", 3000);
    write_checkpoint_json(tmp.path(), "ckpt-mid", "ccc", 2000);

    let mgr = make_manager(tmp.path(), 10);
    let list = mgr.list();

    assert_eq!(list.len(), 3);
    assert_eq!(list[0].id, "ckpt-new");
    assert_eq!(list[1].id, "ckpt-mid");
    assert_eq!(list[2].id, "ckpt-old");
}

#[test]
fn test_restore_success() {
    let tmp = tempfile::tempdir().unwrap();
    write_checkpoint_json(tmp.path(), "ckpt-abc-100", "abc123ff", 100);

    let mgr = make_manager(tmp.path(), 5);
    let ckpt = mgr.restore("ckpt-abc-100").unwrap();

    assert_eq!(ckpt.id, "ckpt-abc-100");
    assert_eq!(ckpt.git_hash, "abc123ff");
    assert_eq!(ckpt.timestamp, 100);
}

#[test]
fn test_restore_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let mgr = make_manager(tmp.path(), 5);

    let result = mgr.restore("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn test_prune_removes_oldest() {
    let tmp = tempfile::tempdir().unwrap();
    write_checkpoint_json(tmp.path(), "ckpt-1", "aaa", 1000);
    write_checkpoint_json(tmp.path(), "ckpt-2", "bbb", 2000);
    write_checkpoint_json(tmp.path(), "ckpt-3", "ccc", 3000);

    let mgr = make_manager(tmp.path(), 2);
    let removed = mgr.prune().unwrap();

    assert_eq!(removed, 1);
    // The oldest (ckpt-1) should be gone.
    assert!(!tmp.path().join("ckpt-1").exists());
    // The two newest should remain.
    assert!(tmp.path().join("ckpt-2").exists());
    assert!(tmp.path().join("ckpt-3").exists());
}

#[test]
fn test_prune_noop_under_limit() {
    let tmp = tempfile::tempdir().unwrap();
    write_checkpoint_json(tmp.path(), "ckpt-1", "aaa", 1000);
    write_checkpoint_json(tmp.path(), "ckpt-2", "bbb", 2000);

    let mgr = make_manager(tmp.path(), 5);
    let removed = mgr.prune().unwrap();

    assert_eq!(removed, 0);
    assert!(tmp.path().join("ckpt-1").exists());
    assert!(tmp.path().join("ckpt-2").exists());
}

#[test]
fn test_checkpoint_serialization_roundtrip() {
    let ckpt = Checkpoint {
        id: "ckpt-test-999".to_string(),
        git_hash: "deadbeef12345678".to_string(),
        timestamp: 1700000000,
        binaries: vec![
            PathBuf::from("/opt/levsha/levsha-chat"),
            PathBuf::from("/opt/levsha/levsha-engine"),
        ],
    };

    let json = serde_json::to_string(&ckpt).unwrap();
    let deserialized: Checkpoint = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, ckpt.id);
    assert_eq!(deserialized.git_hash, ckpt.git_hash);
    assert_eq!(deserialized.timestamp, ckpt.timestamp);
    assert_eq!(deserialized.binaries, ckpt.binaries);
}
