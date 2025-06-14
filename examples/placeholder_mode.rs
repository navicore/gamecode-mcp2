/// Example showing placeholder mode - tool calls replaced with user-friendly text
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
    mcp_client.initialize("placeholder-example", "1.0.0").await?;
    let mcp_client = Arc::new(Mutex::new(mcp_client));

    // Configure with placeholder mode
    let config = ChatIntegrationConfig {
        streaming_mode: StreamingMode::WithPlaceholders {
            placeholder_text: "\n[🔧 Working...]\n".to_string(),
        },
        enhance_system_prompts: true,
        max_tool_rounds: 3,
        instrumentation: InstrumentationConfig {
            log_path: Some("placeholder_debug.log".to_string()),
            log_token_classifications: false,
            log_performance_metrics: true,
        },
    };

    let integration = McpChatIntegration::new(mcp_client, config).await?;

    // Simulate a response with multiple tool calls
    let tokens = vec![
        "I'll help you explore the directory structure.\n\n",
        "{\"tool\": \"list_files\", \"params\": {\"path\": \".\"}}",
        "\n\nNow let me check the src directory:\n\n",
        "{\"tool\": \"list_files\", \"params\": {\"path\": \"./src\"}}",
        "\n\nBased on what I found, here's the project structure:",
    ];
    
    let stream = stream::iter(tokens.into_iter().map(|s| s.to_string()));
    let mut handle = integration.process_streaming_response(stream).await?;

    // Display output with placeholders
    println!("=== User sees this output ===");
    while let Some(token) = handle.token_stream.recv().await {
        match token {
            ProcessedToken::Narrative(text) => {
                print!("{}", text);
            }
            _ => {} // Tool calls are replaced with placeholders
        }
    }
    println!();

    // Show tool results separately
    println!("\n=== Tool executions (backend) ===");
    while let Some(tool) = handle.tool_stream.recv().await {
        println!("✓ {} completed in {}ms", tool.tool_name, tool.execution_time_ms);
    }

    Ok(())
}