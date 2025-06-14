use anyhow::Result;
use gamecode_mcp_host::{McpHost, McpHostConfig, OllamaProvider};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("mcp_host=warn,mcp_client=warn")
        .init();

    // Create and initialize MCP client
    let mut mcp_client = gamecode_mcp_client::McpClient::connect(
        "target/debug/gamecode-mcp2",
        &[
            "--tools-file".to_string(),
            "mcp-host/examples/example_tools.yaml".to_string(),
        ],
    )
    .await?;

    mcp_client.initialize("mcp-host-interactive", "0.1.0").await?;
    let mcp_client = Arc::new(Mutex::new(mcp_client));

    // Create Ollama provider - you can change the model here
    let ollama = OllamaProvider::new("qwen3:14b".to_string());

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

    println!("MCP Host Interactive Example");
    println!("============================");
    println!("Type 'exit' to quit, 'help' for available tools");
    println!();

    loop {
        // Get user input
        print!("> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == "exit" {
            println!("Goodbye!");
            break;
        }

        if input == "help" {
            println!("Available tools:");
            println!("- check_file_exists <path>: Check if a file exists");
            println!("- list_files <path>: List files in a directory");
            println!("- read_file <path>: Read the contents of a file");
            println!();
            continue;
        }

        // Process message
        println!("Thinking...");
        let start = std::time::Instant::now();
        
        match host.process_message(input).await {
            Ok(response) => {
                println!("\nAssistant ({}s): {}\n", start.elapsed().as_secs(), response);
            }
            Err(e) => {
                eprintln!("\nError: {}\n", e);
            }
        }
    }

    Ok(())
}