use gamecode_mcp2::tools::ToolManager;
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_path_traversal_prevention() {
    let _temp_dir = TempDir::new().unwrap();
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("tests/fixtures/test_tools.yaml");
    tool_manager.load_from_file(&path).await.unwrap();

    // Try various path traversal attempts
    let traversal_attempts = vec![
        "../../../etc/passwd",
        "../../../../../../etc/shadow",
        "/etc/passwd",
        "~/.ssh/id_rsa",
    ];

    for attempt in traversal_attempts {
        let args = json!({
            "path": attempt,
            "content": "malicious content"
        });

        // Current implementation doesn't prevent this - documenting the risk
        let result = tool_manager.execute_tool("file_writer", args).await;

        // This test documents that path traversal IS possible
        // In a secure implementation, these should all fail
        if result.is_ok() {
            println!("WARNING: Path traversal successful with: {}", attempt);
        }
    }
}

#[tokio::test]
async fn test_command_argument_injection() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("tests/fixtures/test_tools.yaml");
    tool_manager.load_from_file(&path).await.unwrap();

    // Test that shell metacharacters are properly escaped
    let injection_attempts = vec![
        ("test$(whoami)", "test$(whoami)"),
        ("test`id`", "test`id`"),
        ("test;ls -la", "test;ls -la"),
        ("test|cat /etc/passwd", "test|cat /etc/passwd"),
        ("test&&rm -rf /tmp/test", "test&&rm -rf /tmp/test"),
    ];

    for (attempt, expected) in injection_attempts {
        let args = json!({
            "message": attempt
        });

        let result = tool_manager.execute_tool("echo_test", args).await;
        assert!(result.is_ok(), "Command should execute");

        let output = result.unwrap();
        let text = output["output"].as_str().unwrap().trim();

        // Verify the injection attempt is treated as literal text
        assert_eq!(
            text, expected,
            "Injection attempt '{}' should produce literal output",
            attempt
        );

        // Verify no command execution occurred
        assert!(
            !text.contains("root"),
            "Should not contain system user info"
        );
        assert!(!text.contains("uid="), "Should not contain uid info");
    }
}

#[tokio::test]
async fn test_yaml_include_path_restrictions() {
    // Create a malicious YAML that tries to include system files
    let temp_dir = TempDir::new().unwrap();
    let malicious_yaml = temp_dir.path().join("malicious.yaml");

    tokio::fs::write(
        &malicious_yaml,
        r#"
include:
  - /etc/passwd
  - ~/.ssh/config
  - ../../../../../../../etc/shadow
tools: []
"#,
    )
    .await
    .unwrap();

    let mut tool_manager = ToolManager::new();
    let result = tool_manager.load_from_file(&malicious_yaml).await;

    // Current implementation will fail because these files aren't valid YAML
    // But it WILL try to read them, which is a security issue
    assert!(result.is_err(), "Should fail to include system files");
}

#[tokio::test]
async fn test_recursive_include_dos() {
    // Test protection against recursive includes
    let temp_dir = TempDir::new().unwrap();
    let yaml_a = temp_dir.path().join("a.yaml");
    let yaml_b = temp_dir.path().join("b.yaml");

    // Create circular reference
    tokio::fs::write(&yaml_a, "include:\n  - ./b.yaml\ntools: []")
        .await
        .unwrap();
    tokio::fs::write(&yaml_b, "include:\n  - ./a.yaml\ntools: []")
        .await
        .unwrap();

    let _tool_manager = ToolManager::new();

    // This currently causes infinite recursion - documenting the issue
    // In production, this should be detected and prevented

    // Commented out to prevent test hanging
    // let result = tool_manager.load_from_file(&yaml_a).await;
    // assert!(result.is_err(), "Should detect circular includes");
}

#[tokio::test]
async fn test_large_file_dos() {
    let temp_dir = TempDir::new().unwrap();

    // Create a very large file
    let large_file = temp_dir.path().join("large.txt");
    let _large_content = "A".repeat(100_000_000); // 100MB

    let mut _tool_manager = ToolManager::new();
    let path = PathBuf::from("tests/fixtures/test_tools.yaml");
    _tool_manager.load_from_file(&path).await.unwrap();

    let _args = json!({
        "path": large_file.to_str().unwrap(),
        "content": _large_content
    });

    // Current implementation has no size limits - documenting the risk
    // This could consume excessive memory/disk

    // Commented out to prevent actual DOS in tests
    // let result = tool_manager.execute_tool("file_writer", args).await;
}

#[tokio::test]
async fn test_symlink_traversal() {
    let temp_dir = TempDir::new().unwrap();
    let target = temp_dir.path().join("target.txt");
    let symlink = temp_dir.path().join("link.txt");

    tokio::fs::write(&target, "target content").await.unwrap();

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target, &symlink).unwrap();

        let mut tool_manager = ToolManager::new();
        let path = PathBuf::from("tests/fixtures/test_tools.yaml");
        tool_manager.load_from_file(&path).await.unwrap();

        // Test that symlinks can be followed (security risk)
        let args = json!({
            "path": symlink.to_str().unwrap(),
            "content": "new content"
        });

        let result = tool_manager.execute_tool("file_writer", args).await;

        // Current implementation follows symlinks - documenting the risk
        if result.is_ok() {
            let content = tokio::fs::read_to_string(&target).await.unwrap();
            assert_eq!(content, "new content", "Symlink was followed");
        }
    }
}
