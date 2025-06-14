use anyhow::Result;
use gamecode_mcp_host::{McpHost, McpHostConfig, OllamaProvider};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("mcp_host=debug,mcp_client=debug")
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

    mcp_client.initialize("mcp-host-example", "0.1.0").await?;
    let mcp_client = Arc::new(Mutex::new(mcp_client));

    // Create Ollama provider
    let ollama = OllamaProvider::new("magistral:24b".to_string());

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
    println!();

    // First message - list available tools
    println!("User: What tools do you have available?");
    println!("(Waiting for LLM response...)");
    let start = std::time::Instant::now();
    let response = host
        .process_message("What tools do you have available?")
        .await?;
    println!("Assistant ({}s): {}", start.elapsed().as_secs(), response);
    println!();

    // Second message - use a tool
    println!("User: Can you check if the file README.md exists?");
    println!("(Waiting for LLM response...)");
    let start = std::time::Instant::now();
    let response = host
        .process_message("Can you check if the file README.md exists?")
        .await?;
    println!("Assistant ({}s): {}", start.elapsed().as_secs(), response);
    println!();

    // Third message - multiple tool usage
    println!("User: Please list all markdown files in the current directory and read the first one you find.");
    println!("(Waiting for LLM response...)");
    let start = std::time::Instant::now();
    let response = host.process_message(
        "Please list all markdown files in the current directory and read the first one you find."
    ).await?;
    println!("Assistant ({}s): {}", start.elapsed().as_secs(), response);

    Ok(())
}
