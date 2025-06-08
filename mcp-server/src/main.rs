// Minimal MCP server implementation for auditable LLM-to-system interaction.
// No external text processing - all JSON handling is explicit and traceable.

use anyhow::Result;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info, warn};

mod handlers;
mod protocol;
mod tools;

use handlers::RequestHandler;
use protocol::*;
use tools::ToolManager;

#[tokio::main]
async fn main() -> Result<()> {
    // Tracing to stderr only - stdout is reserved for JSON-RPC protocol
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("mcp_server=info".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    info!("Starting GameCode MCP Server v2...");

    // Tool loading is explicit - no implicit tool discovery
    let mut tool_manager = ToolManager::new();

    // Check for mode override
    if let Ok(mode) = std::env::var("GAMECODE_MODE") {
        info!("Loading tools for mode: {}", mode);
        if let Err(e) = tool_manager.load_mode(&mode).await {
            warn!("Failed to load mode {}: {}", mode, e);
            warn!("Falling back to auto-detection");
            if let Err(e) = tool_manager.detect_and_load_mode().await {
                warn!("Failed to auto-detect mode: {}", e);
                warn!("The server will start but no tools will be available.");
            }
        }
    } else {
        // Try auto-detection first
        if let Err(e) = tool_manager.detect_and_load_mode().await {
            warn!("Failed to auto-detect mode: {}", e);
            // Fall back to default locations
            if let Err(e) = tool_manager.load_from_default_locations().await {
                warn!("Failed to load tools: {}", e);
                warn!("The server will start but no tools will be available.");
            }
        }
    }

    let handler = RequestHandler::new(tool_manager);

    // Stdio is our only transport - no network, no files
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut stdout = stdout;

    info!("MCP server ready, waiting for requests...");

    // Single-threaded message loop - one request at a time
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                info!("Client disconnected");
                break;
            }
            Ok(_) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                debug!("Received: {}", line);

                // Parse as generic Value first - no implicit deserialization
                match serde_json::from_str::<Value>(line) {
                    Ok(value) => {
                        // Explicit request/notification discrimination by id field
                        if value.get("id").is_some() {
                            // It's a request
                            match serde_json::from_value::<JsonRpcRequest>(value) {
                                Ok(request) => {
                                    let response = handler.handle_request(request).await;
                                    let response_str = serde_json::to_string(&response)?;
                                    debug!("Sending: {}", response_str);
                                    stdout.write_all(response_str.as_bytes()).await?;
                                    stdout.write_all(b"\n").await?;
                                    stdout.flush().await?;
                                }
                                Err(e) => {
                                    error!("Invalid request: {}", e);
                                    let error_response = JsonRpcResponse {
                                        jsonrpc: "2.0".to_string(),
                                        id: serde_json::Value::Null,
                                        result: None,
                                        error: Some(JsonRpcError {
                                            code: INVALID_REQUEST,
                                            message: "Invalid request".to_string(),
                                            data: None,
                                        }),
                                    };
                                    let response_str = serde_json::to_string(&error_response)?;
                                    stdout.write_all(response_str.as_bytes()).await?;
                                    stdout.write_all(b"\n").await?;
                                    stdout.flush().await?;
                                }
                            }
                        } else {
                            // It's a notification
                            match serde_json::from_value::<JsonRpcNotification>(value) {
                                Ok(notification) => {
                                    handler.handle_notification(notification).await;
                                }
                                Err(e) => {
                                    error!("Invalid notification: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Parse error: {}", e);
                        let error_response = JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: serde_json::Value::Null,
                            result: None,
                            error: Some(JsonRpcError {
                                code: PARSE_ERROR,
                                message: "Parse error".to_string(),
                                data: None,
                            }),
                        };
                        let response_str = serde_json::to_string(&error_response)?;
                        stdout.write_all(response_str.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                    }
                }
            }
            Err(e) => {
                error!("Read error: {}", e);
                break;
            }
        }
    }

    info!("MCP server shutting down");
    Ok(())
}
