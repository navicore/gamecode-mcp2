use anyhow::Result;
use gamecode_mcp_host::{McpHost, McpHostConfig, OllamaProvider};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("gamecode_mcp_host=info,gamecode_mcp_client=info")
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

    mcp_client.initialize("mcp-host-quick", "0.1.0").await?;
    let mcp_client = Arc::new(Mutex::new(mcp_client));

    // Create Ollama provider with a faster model
    let ollama = OllamaProvider::new("llama3.2:1b".to_string()); // Much faster small model

    // Configure MCP host
    let config = McpHostConfig {
        max_retries: 1,
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

    println!("MCP Host Quick Test");
    println!("==================\n");

    // Test 1: Simple tool usage
    println!("Test 1: Check if README.md exists");
    let start = std::time::Instant::now();
    match host.process_message("Check if README.md exists using the check_file_exists tool").await {
        Ok(response) => println!("Response ({}s): {}\n", start.elapsed().as_secs(), response),
        Err(e) => println!("Error: {}\n", e),
    }

    // Test 2: Multiple tool usage
    println!("Test 2: List files and check existence");
    let start = std::time::Instant::now();
    match host.process_message("Use list_files tool on '.' directory, then check if the first file exists").await {
        Ok(response) => println!("Response ({}s): {}\n", start.elapsed().as_secs(), response),
        Err(e) => println!("Error: {}\n", e),
    }

    Ok(())
}