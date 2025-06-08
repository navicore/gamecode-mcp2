// Request handler - validates all LLM requests before execution.
// No request reaches tool execution without explicit validation.

use anyhow::Result;
use serde_json::Value;
use tracing::{debug, error, info};

use crate::protocol::*;
use crate::tools::ToolManager;

pub struct RequestHandler {
    tool_manager: ToolManager,
}

impl RequestHandler {
    pub fn new(tool_manager: ToolManager) -> Self {
        Self { tool_manager }
    }

    // Request dispatch - only these three methods exist, nothing else
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        debug!("Handling request: {} (id: {})", request.method, request.id);

        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params).await,
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tools_call(request.params).await,
            _ => Err(JsonRpcError {
                code: METHOD_NOT_FOUND,
                message: format!("Method '{}' not found", request.method),
                data: None,
            }),
        };

        match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(value),
                error: None,
            },
            Err(error) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(error),
            },
        }
    }

    pub async fn handle_notification(&self, notification: JsonRpcNotification) {
        debug!("Handling notification: {}", notification.method);

        match notification.method.as_str() {
            "notifications/initialized" => {
                info!("Client initialized");
            }
            "notifications/cancelled" => {
                info!("Request cancelled");
            }
            _ => {
                debug!("Unknown notification: {}", notification.method);
            }
        }
    }

    // Initialize - validate client capabilities, no negotiation
    async fn handle_initialize(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let _params: InitializeParams = if let Some(p) = params {
            serde_json::from_value(p).map_err(|e| JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Invalid initialize params: {}", e),
                data: None,
            })?
        } else {
            return Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing initialize params".to_string(),
                data: None,
            });
        };

        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: ToolsCapability {},
            },
            server_info: ServerInfo {
                name: "gamecode-mcp2".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(
                "GameCode MCP Server v2 - Direct tool integration. Configure tools in tools.yaml"
                    .to_string(),
            ),
        };

        Ok(serde_json::to_value(result).unwrap())
    }

    // List tools - LLM sees only what we explicitly configured
    async fn handle_tools_list(&self) -> Result<Value, JsonRpcError> {
        let tools = self.tool_manager.get_mcp_tools();

        let result = ListToolsResult { tools };

        Ok(serde_json::to_value(result).unwrap())
    }

    // Tool execution - validate params, then delegate to tool manager
    async fn handle_tools_call(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params: CallToolParams = if let Some(p) = params {
            serde_json::from_value(p).map_err(|e| JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Invalid tool call params: {}", e),
                data: None,
            })?
        } else {
            return Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing tool call params".to_string(),
                data: None,
            });
        };

        // Execute only configured tools with validated parameters
        match self
            .tool_manager
            .execute_tool(&params.name, params.arguments)
            .await
        {
            Ok(result) => {
                let response = CallToolResult {
                    content: vec![ContentBlock::Text {
                        text: serde_json::to_string(&result).unwrap_or_else(|_| "null".to_string()),
                    }],
                    is_error: None,
                };

                Ok(serde_json::to_value(response).unwrap())
            }
            Err(e) => {
                error!("Tool execution failed: {}", e);

                let response = CallToolResult {
                    content: vec![ContentBlock::Text {
                        text: format!("Error: {}", e),
                    }],
                    is_error: Some(true),
                };

                Ok(serde_json::to_value(response).unwrap())
            }
        }
    }
}
