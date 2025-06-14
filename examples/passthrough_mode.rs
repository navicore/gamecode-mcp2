/// Example demonstrating passthrough mode - no tool interception
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
    mcp_client.initialize("passthrough-example", "1.0.0").await?;
    let mcp_client = Arc::new(Mutex::new(mcp_client));

    // Configure for passthrough mode
    let config = ChatIntegrationConfig {
        streaming_mode: StreamingMode::Passthrough,
        enhance_system_prompts: false, // Don't modify prompts
        max_tool_rounds: 0, // No tool execution
        instrumentation: InstrumentationConfig {
            log_path: Some("passthrough_debug.log".to_string()),
            log_token_classifications: true,
            log_performance_metrics: true,
        },
    };

    let integration = McpChatIntegration::new(mcp_client, config).await?;

    // Simulate a streaming response
    let tokens = vec![
        "I'll analyze your request.\n\n",
        "{\"tool\": \"list_files\", \"params\": {\"path\": \".\"}}",
        "\n\nBased on the files, here's my recommendation.",
    ];
    
    let stream = stream::iter(tokens.into_iter().map(|s| s.to_string()));
    let mut handle = integration.process_streaming_response(stream).await?;

    // In passthrough mode, everything is narrative
    println!("=== Passthrough Output ===");
    while let Some(token) = handle.token_stream.recv().await {
        match token {
            ProcessedToken::Narrative(text) => {
                print!("{}", text); // Everything passes through
            }
            _ => {
                // Should not happen in passthrough mode
                println!("\n[Unexpected token type]");
            }
        }
    }
    println!();

    // No tools are executed in passthrough mode
    if let Some(tool) = handle.tool_stream.recv().await {
        println!("Warning: Tool executed in passthrough mode: {}", tool.tool_name);
    }

    Ok(())
}