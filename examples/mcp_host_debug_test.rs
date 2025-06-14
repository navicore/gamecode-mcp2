use anyhow::Result;
use gamecode_mcp_host::{McpHost, McpHostConfig, OllamaProvider};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("gamecode_mcp_host=debug,gamecode_mcp_client=info")
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

    mcp_client.initialize("debug-complex", "0.1.0").await?;
    let mcp_client = Arc::new(Mutex::new(mcp_client));

    // Create Ollama provider
    let ollama = OllamaProvider::new("qwen3:14b".to_string());

    // Configure MCP host
    let config = McpHostConfig {
        max_retries: 0,
        retry_delay_ms: 500,
        temperature_reduction: 0.1,
        safety_constraints: Default::default(),
    };

    // Build MCP host
    let mut host = McpHost::builder()
        .with_llm_provider(Box::new(ollama))
        .with_mcp_client(mcp_client)
        .with_config(config)
        .build()?;

    println!("Debug Complex Request");
    println!("====================\n");

    // The problematic request
    println!("User: Please list all markdown files in the current directory and read the first one you find.");
    let start = std::time::Instant::now();
    match host.process_message("Please list all markdown files in the current directory and read the first one you find.").await {
        Ok(response) => {
            println!("\nCompleted in {}s", start.elapsed().as_secs());
            println!("Assistant: {}", response);
        }
        Err(e) => {
            println!("\nFailed after {}s: {:?}", start.elapsed().as_secs(), e);
        }
    }

    Ok(())
}