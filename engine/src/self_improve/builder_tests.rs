use super::*;
use std::path::PathBuf;

#[test]
fn test_build_target_from_str_chat_shell() {
    for input in &["chat-shell", "chat_shell", "shell", "Chat-Shell", "SHELL"] {
        let target = BuildTarget::from_str(input);
        assert!(
            matches!(target, BuildTarget::ChatShell),
            "expected ChatShell for input '{}'",
            input
        );
    }
}

#[test]
fn test_build_target_from_str_engine() {
    for input in &["engine", "Engine", "ENGINE"] {
        let target = BuildTarget::from_str(input);
        assert!(
            matches!(target, BuildTarget::Engine),
            "expected Engine for input '{}'",
            input
        );
    }
}

#[test]
fn test_build_target_from_str_full() {
    for input in &["full", "workspace", "all", "unknown", ""] {
        let target = BuildTarget::from_str(input);
        assert!(
            matches!(target, BuildTarget::Full),
            "expected Full for input '{}'",
            input
        );
    }
}

#[test]
fn test_package_arg_chat_shell() {
    let target = BuildTarget::ChatShell;
    assert_eq!(target.package_arg(), Some("levsha-chat"));
}

#[test]
fn test_package_arg_engine() {
    let target = BuildTarget::Engine;
    assert_eq!(target.package_arg(), Some("levsha-engine"));
}

#[test]
fn test_package_arg_full() {
    let target = BuildTarget::Full;
    assert_eq!(target.package_arg(), None);
}

#[test]
fn test_parse_cargo_output_extracts_errors() {
    let stderr = "\
error[E0308]: mismatched types
  --> src/main.rs:5:14
error: aborting due to previous error
";
    let (warnings, errors) = parse_cargo_output(stderr);
    assert!(warnings.is_empty());
    assert_eq!(errors.len(), 2);
    assert!(errors[0].contains("error[E0308]"));
    assert!(errors[1].contains("error: aborting"));
}

#[test]
fn test_parse_cargo_output_extracts_warnings() {
    let stderr = "\
warning: unused variable: `x`
  --> src/main.rs:3:9
warning[unused_imports]: unused import
";
    let (warnings, errors) = parse_cargo_output(stderr);
    assert_eq!(warnings.len(), 2);
    assert!(errors.is_empty());
    assert!(warnings[0].contains("warning: unused variable"));
    assert!(warnings[1].contains("warning[unused_imports]"));
}

#[test]
fn test_parse_cargo_output_mixed() {
    let stderr = "\
   Compiling levsha-engine v0.1.0
warning: unused variable: `x`
error[E0425]: cannot find value `y`
warning: function is never used
error: could not compile
";
    let (warnings, errors) = parse_cargo_output(stderr);
    assert_eq!(warnings.len(), 2);
    assert_eq!(errors.len(), 2);
}

#[test]
fn test_parse_cargo_output_empty() {
    let (warnings, errors) = parse_cargo_output("");
    assert!(warnings.is_empty());
    assert!(errors.is_empty());
}

#[test]
fn test_artifact_path_chat_shell() {
    let root = PathBuf::from("/tmp/levsha");
    let target_dir = root.join("target/release");
    let expected = target_dir.join("levsha-chat");

    // Verify the expected artifact name matches the package.
    let target = BuildTarget::ChatShell;
    assert_eq!(target.package_arg(), Some("levsha-chat"));
    assert_eq!(expected.file_name().unwrap(), "levsha-chat");
}

#[test]
fn test_artifact_path_engine() {
    let root = PathBuf::from("/tmp/levsha");
    let target_dir = root.join("target/release");
    let expected = target_dir.join("levsha-engine");

    let target = BuildTarget::Engine;
    assert_eq!(target.package_arg(), Some("levsha-engine"));
    assert_eq!(expected.file_name().unwrap(), "levsha-engine");
}
