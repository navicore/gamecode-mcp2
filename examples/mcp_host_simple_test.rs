use anyhow::Result;
use gamecode_mcp_host::{McpHost, McpHostConfig, OllamaProvider};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    // Minimal logging
    tracing_subscriber::fmt()
        .with_env_filter("gamecode_mcp_host=warn,gamecode_mcp_client=warn")
        .init();

    // Create and initialize MCP client
    let mut mcp_client = gamecode_mcp_client::McpClient::connect(
        "target/debug/gamecode-mcp2",
        &[
            "--tools-file".to_string(),
            "examples/mcp_host_example_tools.yaml".to_string(),
        ],
    )
    .await?;

    mcp_client.initialize("quick-test", "0.1.0").await?;
    let mcp_client = Arc::new(Mutex::new(mcp_client));

    // Use magistral
    let ollama = OllamaProvider::new("magistral:24b".to_string());

    // Configure MCP host with aggressive limits
    let mut config = McpHostConfig::default();
    config.max_retries = 0;
    config.safety_constraints.max_tokens_per_request = 50; // Very low for testing

    // Build MCP host
    let mut host = McpHost::builder()
        .with_llm_provider(Box::new(ollama))
        .with_mcp_client(mcp_client)
        .with_config(config)
        .build()?;

    println!("Quick magistral test with 50 token limit\n");

    // Simple request
    println!("User: What tools do you have?");
    let start = std::time::Instant::now();
    match host.process_message("What tools do you have?").await {
        Ok(response) => {
            println!("\nTime: {}s", start.elapsed().as_secs_f32());
            println!("Assistant: {}", response);
        }
        Err(e) => {
            println!("\nError: {:?}", e);
        }
    }

    Ok(())
}