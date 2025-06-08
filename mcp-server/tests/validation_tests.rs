use mcp_server::tools::ToolManager;
use serde_json::json;
use std::path::PathBuf;

#[tokio::test]
async fn test_path_validation_rejection() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("../examples/tools/secured.yaml");
    tool_manager.load_from_file(&path).await.unwrap();
    
    // Try path traversal - should be rejected
    let args = json!({
        "file": "../../../etc/passwd"
    });
    
    let result = tool_manager.execute_tool("safe_file_reader", args).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Path traversal detected"));
}

#[tokio::test]
async fn test_absolute_path_rejection() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("../examples/tools/secured.yaml");
    tool_manager.load_from_file(&path).await.unwrap();
    
    // Try absolute path - should be rejected
    let args = json!({
        "file": "/etc/passwd"
    });
    
    let result = tool_manager.execute_tool("safe_file_reader", args).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Absolute paths not allowed"));
}

#[tokio::test]
async fn test_validation_allows_safe_paths() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("../examples/tools/secured.yaml");
    tool_manager.load_from_file(&path).await.unwrap();
    
    // Safe relative path should work
    let args = json!({
        "file": "README.md"
    });
    
    let result = tool_manager.execute_tool("safe_file_reader", args).await;
    // This might fail if README.md doesn't exist, but shouldn't fail validation
    if result.is_err() {
        let err = result.unwrap_err().to_string();
        assert!(!err.contains("Path traversal"));
        assert!(!err.contains("Absolute paths"));
    }
}

#[tokio::test]
async fn test_unrestricted_tool_works() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("../examples/tools/secured.yaml");
    tool_manager.load_from_file(&path).await.unwrap();
    
    // Unrestricted echo should accept anything
    let args = json!({
        "message": "../../../etc/passwd; cat /etc/shadow"
    });
    
    let result = tool_manager.execute_tool("unrestricted_echo", args).await;
    assert!(result.is_ok());
    
    // But the content should be literal (no execution)
    let output = result.unwrap();
    assert!(output["output"].as_str().unwrap().contains("../../../etc/passwd; cat /etc/shadow"));
}

#[tokio::test]
async fn test_null_byte_rejection() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("../examples/tools/secured.yaml");
    tool_manager.load_from_file(&path).await.unwrap();
    
    // Try null byte injection
    let args = json!({
        "file": "file.txt\0.bypass"
    });
    
    let result = tool_manager.execute_tool("safe_file_reader", args).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("null byte"));
}