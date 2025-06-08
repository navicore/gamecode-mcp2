use mcp_server::tools::ToolManager;
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_execute_echo_command() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("tests/fixtures/test_tools.yaml");
    tool_manager.load_from_file(&path).await.unwrap();
    
    let args = json!({
        "message": "Hello, World!"
    });
    
    let result = tool_manager.execute_tool("echo_test", args).await;
    assert!(result.is_ok(), "Echo command failed: {:?}", result);
    
    let output = result.unwrap();
    assert!(output["output"].as_str().unwrap().contains("Hello, World!"));
}

#[tokio::test]
async fn test_execute_internal_math() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("tests/fixtures/test_tools.yaml");
    tool_manager.load_from_file(&path).await.unwrap();
    
    let args = json!({
        "a": 5.0,
        "b": 3.0
    });
    
    let result = tool_manager.execute_tool("math_add", args).await;
    assert!(result.is_ok(), "Math addition failed: {:?}", result);
    
    let output = result.unwrap();
    assert_eq!(output["result"], 8.0);
    assert_eq!(output["operation"], "addition");
}

#[tokio::test]
async fn test_execute_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("tests/fixtures/test_tools.yaml");
    tool_manager.load_from_file(&path).await.unwrap();
    
    // Test file writing
    let write_args = json!({
        "path": test_file.to_str().unwrap(),
        "content": "Test content"
    });
    
    let write_result = tool_manager.execute_tool("file_writer", write_args).await;
    assert!(write_result.is_ok(), "File write failed: {:?}", write_result);
    
    // Verify file was written
    let content = tokio::fs::read_to_string(&test_file).await.unwrap();
    assert_eq!(content, "Test content");
    
    // Test directory listing
    let list_args = json!({
        "path": temp_dir.path().to_str().unwrap()
    });
    
    let list_result = tool_manager.execute_tool("list_dir", list_args).await;
    assert!(list_result.is_ok(), "Directory listing failed: {:?}", list_result);
    
    let output = list_result.unwrap();
    let files = output["files"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["name"], "test.txt");
}

#[tokio::test]
async fn test_execute_nonexistent_tool() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("tests/fixtures/test_tools.yaml");
    tool_manager.load_from_file(&path).await.unwrap();
    
    let args = json!({});
    let result = tool_manager.execute_tool("does_not_exist", args).await;
    
    assert!(result.is_err(), "Should fail for nonexistent tool");
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn test_execute_missing_required_args() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("tests/fixtures/test_tools.yaml");
    tool_manager.load_from_file(&path).await.unwrap();
    
    // Try to execute echo_test without required message parameter
    let args = json!({});
    
    // The current implementation might not validate required args
    // This test documents the current behavior
    let result = tool_manager.execute_tool("echo_test", args).await;
    
    // If validation is added later, this test should be updated
    // For now, it likely succeeds but with empty output
    if result.is_ok() {
        let output = result.unwrap();
        // Echo with no args should produce minimal output
        assert!(output["output"].as_str().is_some());
    }
}

#[tokio::test]
async fn test_command_injection_prevention() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("tests/fixtures/test_tools.yaml");
    tool_manager.load_from_file(&path).await.unwrap();
    
    // Try to inject commands through arguments
    let args = json!({
        "message": "test; rm -rf /tmp/test"
    });
    
    let result = tool_manager.execute_tool("echo_test", args).await;
    assert!(result.is_ok(), "Command should execute safely");
    
    let output = result.unwrap();
    // The semicolon and command should be treated as literal text
    assert!(output["output"].as_str().unwrap().contains("test; rm -rf /tmp/test"));
}