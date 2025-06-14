/// Minimal example showing basic streaming integration
use anyhow::Result;
use futures::stream;
use gamecode_mcp_host::{
    McpChatIntegration, ChatIntegrationConfig, StreamingMode,
    ProcessedToken, InstrumentationConfig,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize MCP client
    let mut mcp_client = gamecode_mcp_client::McpClient::connect(
        "target/debug/gamecode-mcp2",
        &["--tools-file".to_string(), "tools.yaml".to_string()],
    ).await?;
    mcp_client.initialize("minimal-example", "1.0.0").await?;
    let mcp_client = Arc::new(Mutex::new(mcp_client));

    // Minimal configuration
    let config = ChatIntegrationConfig {
        streaming_mode: StreamingMode::SmartBuffering { max_buffer_chars: 150 },
        enhance_system_prompts: true,
        max_tool_rounds: 3,
        instrumentation: InstrumentationConfig {
            log_path: None,
            log_token_classifications: false,
            log_performance_metrics: false,
        },
    };

    let integration = McpChatIntegration::new(mcp_client, config).await?;

    // Simulate a streaming response that includes a tool call
    let tokens = vec![
        "Let me check the files.\n\n",
        "{\"tool\": \"list_files\", \"params\": {\"path\": \".\"}}",
        "\n\nHere are the files I found:",
    ];
    
    let stream = stream::iter(tokens.into_iter().map(|s| s.to_string()));
    let mut handle = integration.process_streaming_response(stream).await?;

    // Process tokens
    println!("=== Tokens received ===");
    while let Some(token) = handle.token_stream.recv().await {
        match token {
            ProcessedToken::Narrative(text) => {
                println!("NARRATIVE: {}", text);
            }
            ProcessedToken::ToolCall(json) => {
                println!("TOOL CALL (hidden): {}", json);
            }
            ProcessedToken::Buffered(text) => {
                println!("BUFFERED: {}", text);
            }
        }
    }

    // Process tool executions
    println!("\n=== Tools executed ===");
    while let Some(tool) = handle.tool_stream.recv().await {
        println!("Tool: {} ({}ms)", tool.tool_name, tool.execution_time_ms);
        println!("Result: {}", serde_json::to_string_pretty(&tool.result)?);
    }

    Ok(())
}