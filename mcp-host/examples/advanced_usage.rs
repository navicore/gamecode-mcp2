use anyhow::Result;
use mcp_host::{McpHost, McpHostConfig, OllamaProvider, SafetyConfig};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("mcp_host=debug")
        .init();

    // Model selection based on availability
    let models = vec!["llama3.1:70b", "llama3.1:8b", "mistral:7b-instruct", "qwen2.5-coder"];
    
    println!("Available models:");
    for (i, model) in models.iter().enumerate() {
        println!("{}. {}", i + 1, model);
    }
    
    print!("Select a model (1-{}): ", models.len());
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let selection: usize = input.trim().parse().unwrap_or(1);
    let model = models.get(selection - 1).unwrap_or(&models[0]).to_string();
    
    println!("Using model: {}", model);

    // Create MCP client
    let mut mcp_client = mcp_client::McpClient::connect(
        "../mcp-server/target/debug/mcp-server",
        &["--tools".to_string(), "../examples/example_tools.yaml".to_string()],
    )
    .await?;
    
    mcp_client.initialize("mcp-host-advanced", "0.1.0").await?;
    let mcp_client = Arc::new(mcp_client);

    // Create Ollama provider with custom configuration
    let ollama = OllamaProvider::with_config(
        model,
        "http://localhost:11434",
        std::time::Duration::from_secs(300), // 5 minute timeout for larger models
    );

    // Configure MCP host with strict safety constraints
    let config = McpHostConfig {
        max_retries: 5,
        retry_delay_ms: 2000,
        temperature_reduction: 0.15,
        safety_constraints: SafetyConfig {
            max_tokens_per_request: 2048,
            max_tools_per_request: 3,
            rate_limit_per_minute: 20,
            blocked_tool_patterns: vec![
                "rm".to_string(),
                "delete".to_string(),
                "drop".to_string(),
            ],
        },
    };

    // Build MCP host
    let mut host = McpHost::builder()
        .with_llm_provider(Box::new(ollama))
        .with_mcp_client(mcp_client)
        .with_config(config)
        .build()?;

    println!("\nMCP Host Advanced Example");
    println!("========================");
    println!("Type 'exit' to quit, 'clear' to clear conversation history");
    println!();

    // Interactive loop
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        print!("> ");
        std::io::Write::flush(&mut std::io::stdout())?;
        
        line.clear();
        reader.read_line(&mut line).await?;
        let input = line.trim();

        match input {
            "exit" => break,
            "clear" => {
                // In a real implementation, we'd add a clear method to ConversationManager
                println!("Conversation history cleared.");
                continue;
            }
            "" => continue,
            _ => {
                match host.process_message(input).await {
                    Ok(response) => {
                        println!("\nAssistant: {}\n", response);
                    }
                    Err(e) => {
                        eprintln!("\nError: {}\n", e);
                        eprintln!("Tip: The model may not support tool calls or the format may be incorrect.");
                    }
                }
            }
        }
    }

    println!("Goodbye!");
    Ok(())
}