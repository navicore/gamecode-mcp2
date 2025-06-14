use anyhow::Result;
use gamecode_mcp_host::{McpHost, McpHostConfig, LlmProvider, LlmRequest, LlmResponse, OllamaProvider};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;
use async_trait::async_trait;

// Wrapper to count LLM requests
struct CountingLlmProvider {
    inner: Box<dyn LlmProvider>,
    request_count: Arc<StdMutex<usize>>,
}

impl CountingLlmProvider {
    fn new(inner: Box<dyn LlmProvider>) -> Self {
        Self {
            inner,
            request_count: Arc::new(StdMutex::new(0)),
        }
    }
    
    fn get_count(&self) -> usize {
        *self.request_count.lock().unwrap()
    }
}

#[async_trait]
impl LlmProvider for CountingLlmProvider {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse> {
        let count = {
            let mut c = self.request_count.lock().unwrap();
            *c += 1;
            *c
        };
        println!("\n[LLM REQUEST #{}]", count);
        println!("Temperature: {}", request.temperature);
        println!("Max tokens: {:?}", request.max_tokens);
        println!("Prompt preview (first 200 chars): {}", 
            request.prompt.chars().take(200).collect::<String>());
        
        let start = std::time::Instant::now();
        let result = self.inner.generate(request).await;
        println!("Response time: {}s", start.elapsed().as_secs());
        
        result
    }
    
    fn name(&self) -> &str {
        self.inner.name()
    }
    
    fn supports_tools(&self) -> bool {
        self.inner.supports_tools()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
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

    mcp_client.initialize("trace-requests", "0.1.0").await?;
    let mcp_client = Arc::new(Mutex::new(mcp_client));

    // Create wrapped Ollama provider
    let ollama = gamecode_mcp_host::OllamaProvider::new("qwen3:14b".to_string());
    let counting_provider = CountingLlmProvider::new(Box::new(ollama));

    // Configure MCP host
    let config = McpHostConfig {
        max_retries: 0,
        retry_delay_ms: 500,
        temperature_reduction: 0.1,
        safety_constraints: Default::default(),
    };

    // Build MCP host
    let mut host = McpHost::builder()
        .with_llm_provider(Box::new(counting_provider))
        .with_mcp_client(mcp_client)
        .with_config(config)
        .build()?;

    println!("Tracing LLM Requests");
    println!("===================\n");

    // Multi-tool request
    println!("User: List files in current directory and check if README.md exists");
    let start = std::time::Instant::now();
    
    match host.process_message("List files in current directory and check if README.md exists").await {
        Ok(response) => {
            println!("\n=== FINAL RESULT ===");
            println!("Total time: {}s", start.elapsed().as_secs());
            println!("Response: {}", response);
        }
        Err(e) => {
            println!("\nError: {:?}", e);
        }
    }

    Ok(())
}