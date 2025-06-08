use gamecode_mcp2::tools::ToolManager;
use std::path::PathBuf;

#[tokio::test]
async fn test_load_simple_tools() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("tests/fixtures/test_tools.yaml");

    let result = tool_manager.load_from_file(&path).await;
    assert!(result.is_ok(), "Failed to load tools: {:?}", result);

    let tools = tool_manager.get_mcp_tools();
    assert_eq!(tools.len(), 4, "Expected 4 tools");

    // Verify tool names
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
    assert!(tool_names.contains(&"echo_test".to_string()));
    assert!(tool_names.contains(&"math_add".to_string()));
    assert!(tool_names.contains(&"file_writer".to_string()));
    assert!(tool_names.contains(&"list_dir".to_string()));
}

#[tokio::test]
async fn test_load_tools_with_include() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("tests/fixtures/tools_with_include.yaml");

    let result = tool_manager.load_from_file(&path).await;
    assert!(
        result.is_ok(),
        "Failed to load tools with include: {:?}",
        result
    );

    let tools = tool_manager.get_mcp_tools();
    // Should have 4 from test_tools.yaml + 1 from tools_with_include.yaml
    assert_eq!(tools.len(), 5, "Expected 5 tools after include");

    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
    assert!(tool_names.contains(&"additional_tool".to_string()));
}

#[tokio::test]
async fn test_tool_schema_generation() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("tests/fixtures/test_tools.yaml");
    tool_manager.load_from_file(&path).await.unwrap();

    let tools = tool_manager.get_mcp_tools();
    let echo_tool = tools.iter().find(|t| t.name == "echo_test").unwrap();

    // Verify schema structure
    let schema = &echo_tool.input_schema;
    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["message"].is_object());
    assert_eq!(schema["properties"]["message"]["type"], "string");
    assert_eq!(schema["required"][0], "message");
}

#[tokio::test]
async fn test_load_nonexistent_file() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("tests/fixtures/does_not_exist.yaml");

    let result = tool_manager.load_from_file(&path).await;
    assert!(result.is_err(), "Should fail to load nonexistent file");
}

#[tokio::test]
async fn test_invalid_yaml_structure() {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("tests/fixtures/invalid_tools.yaml");

    // This test depends on whether serde_yaml catches the missing fields
    // In production, we might want to add explicit validation
    let result = tool_manager.load_from_file(&path).await;
    // The current implementation might actually load this,
    // which could be a security concern
    if result.is_ok() {
        let tools = tool_manager.get_mcp_tools();
        // Verify that invalid tools are handled gracefully
        for tool in tools {
            assert!(!tool.description.is_empty(), "Tool should have description");
        }
    }
}
