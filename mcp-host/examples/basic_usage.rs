use anyhow::Result;
use mcp_host::{McpHost, McpHostConfig, OllamaProvider};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("mcp_host=debug,mcp_client=debug")
        .init();

    // Create and initialize MCP client
    let mut mcp_client = mcp_client::McpClient::connect(
        "../mcp-server/target/debug/mcp-server",
        &["--tools".to_string(), "../examples/example_tools.yaml".to_string()],
    )
    .await?;
    
    mcp_client.initialize("mcp-host-example", "0.1.0").await?;
    let mcp_client = Arc::new(mcp_client);

    // Create Ollama provider
    let ollama = OllamaProvider::new("llama3.1:8b".to_string());

    // Configure MCP host
    let config = McpHostConfig {
        max_retries: 3,
        retry_delay_ms: 1000,
        temperature_reduction: 0.1,
        safety_constraints: Default::default(),
    };

    // Build MCP host
    let mut host = McpHost::builder()
        .with_llm_provider(Box::new(ollama))
        .with_mcp_client(mcp_client)
        .with_config(config)
        .build()?;

    // Example conversation
    println!("MCP Host Example");
    println!("================");
    
    // First message - list available tools
    let response = host.process_message("What tools do you have available?").await?;
    println!("User: What tools do you have available?");
    println!("Assistant: {}", response);
    println!();

    // Second message - use a tool
    let response = host.process_message("Can you check if the file README.md exists?").await?;
    println!("User: Can you check if the file README.md exists?");
    println!("Assistant: {}", response);
    println!();

    // Third message - multiple tool usage
    let response = host.process_message(
        "Please list all markdown files in the current directory and read the first one you find."
    ).await?;
    println!("User: Please list all markdown files in the current directory and read the first one you find.");
    println!("Assistant: {}", response);

    Ok(())
}