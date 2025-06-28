use gamecode_mcp2::handlers::RequestHandler;
use gamecode_mcp2::protocol::*;
use gamecode_mcp2::tools::ToolManager;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;

async fn setup_handler() -> RequestHandler {
    let mut tool_manager = ToolManager::new();
    let path = PathBuf::from("tests/fixtures/test_tools.yaml");
    tool_manager.load_from_file(&path).await.unwrap();
    RequestHandler::new(tool_manager, HashMap::new())
}

#[tokio::test]
async fn test_initialize_request() {
    let handler = setup_handler().await;

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(1),
        method: "initialize".to_string(),
        params: Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        })),
    };

    let response = handler.handle_request(request).await;

    assert!(
        response.error.is_none(),
        "Initialize failed: {:?}",
        response.error
    );
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert!(result["capabilities"]["tools"].is_object());
    assert_eq!(result["serverInfo"]["name"], "gamecode-mcp2");
}

#[tokio::test]
async fn test_tools_list_request() {
    let handler = setup_handler().await;

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(2),
        method: "tools/list".to_string(),
        params: None,
    };

    let response = handler.handle_request(request).await;

    assert!(
        response.error.is_none(),
        "List tools failed: {:?}",
        response.error
    );
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    let tools = result["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 4, "Expected 4 tools from test fixture");

    // Verify tool structure
    for tool in tools {
        assert!(tool["name"].is_string());
        assert!(tool["description"].is_string());
        assert!(tool["inputSchema"].is_object());
    }
}

#[tokio::test]
async fn test_tools_call_request() {
    let handler = setup_handler().await;

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(3),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "math_add",
            "arguments": {
                "a": 10,
                "b": 20
            }
        })),
    };

    let response = handler.handle_request(request).await;

    assert!(
        response.error.is_none(),
        "Tool call failed: {:?}",
        response.error
    );
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result["content"].is_array());

    let content = &result["content"][0];
    assert_eq!(content["type"], "text");

    // Parse the text content to verify the result
    let text = content["text"].as_str().unwrap();
    let tool_result: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(tool_result["result"], 30.0);
}

#[tokio::test]
async fn test_invalid_method() {
    let handler = setup_handler().await;

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(4),
        method: "invalid/method".to_string(),
        params: None,
    };

    let response = handler.handle_request(request).await;

    assert!(response.error.is_some());
    assert_eq!(response.error.unwrap().code, METHOD_NOT_FOUND);
}

#[tokio::test]
async fn test_missing_params() {
    let handler = setup_handler().await;

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(5),
        method: "initialize".to_string(),
        params: None, // Missing required params
    };

    let response = handler.handle_request(request).await;

    assert!(response.error.is_some());
    assert_eq!(response.error.unwrap().code, INVALID_PARAMS);
}

#[tokio::test]
async fn test_tool_call_nonexistent_tool() {
    let handler = setup_handler().await;

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(6),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "nonexistent_tool",
            "arguments": {}
        })),
    };

    let response = handler.handle_request(request).await;

    assert!(
        response.error.is_none(),
        "Should return error in content, not JSON-RPC error"
    );
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert_eq!(result["isError"], true);

    let content = &result["content"][0];
    assert!(content["text"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_notification_handling() {
    let handler = setup_handler().await;

    let notification = JsonRpcNotification {
        jsonrpc: "2.0".to_string(),
        method: "notifications/initialized".to_string(),
        params: None,
    };

    // Notifications don't return responses, just verify no panic
    handler.handle_notification(notification).await;
}
