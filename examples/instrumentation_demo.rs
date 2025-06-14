/// Example demonstrating debug instrumentation features
use anyhow::Result;
use futures::stream;
use gamecode_mcp_host::{
    McpChatIntegration, ChatIntegrationConfig, StreamingMode,
    ProcessedToken, InstrumentationConfig,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize MCP client
    let mut mcp_client = gamecode_mcp_client::McpClient::connect(
        "target/debug/gamecode-mcp2",
        &["--tools-file".to_string(), "tools.yaml".to_string()],
    ).await?;
    mcp_client.initialize("instrumentation-demo", "1.0.0").await?;
    let mcp_client = Arc::new(Mutex::new(mcp_client));

    // Enable full instrumentation
    let config = ChatIntegrationConfig {
        streaming_mode: StreamingMode::SmartBuffering { max_buffer_chars: 100 },
        enhance_system_prompts: true,
        max_tool_rounds: 3,
        instrumentation: InstrumentationConfig {
            log_path: Some("full_instrumentation.jsonl".to_string()),
            log_token_classifications: true,
            log_performance_metrics: true,
        },
    };

    let integration = McpChatIntegration::new(mcp_client, config).await?;

    // Test prompt enhancement
    println!("=== Testing Prompt Enhancement ===");
    let base_prompt = "You are a helpful assistant.";
    let enhanced = integration.enhance_system_prompt(base_prompt).await?;
    println!("Base prompt length: {}", base_prompt.len());
    println!("Enhanced prompt length: {}", enhanced.len());
    println!("First 200 chars: {}...\n", &enhanced[..200.min(enhanced.len())]);

    // Simulate various streaming patterns
    println!("=== Testing Token Classification ===");
    
    // Pattern 1: Clean tool call
    let tokens1 = vec![
        "Checking files now.\n\n",
        "{\"tool\": \"list_files\", ",
        "\"params\": {\"path\": \".\"}}",
        "\n\nFound the following files:",
    ];
    
    process_pattern(&integration, "Clean Tool Call", tokens1).await?;
    sleep(Duration::from_millis(100)).await;

    // Pattern 2: Tool call with narrative mixed in
    let tokens2 = vec![
        "Let me list the files for you. ",
        "I'll use the list_files tool: {\"tool\":",
        " \"list_files\", \"params\":",
        " {\"path\": \".\"}} to check.",
    ];
    
    process_pattern(&integration, "Mixed Narrative", tokens2).await?;
    sleep(Duration::from_millis(100)).await;

    // Pattern 3: Multiple tools
    let tokens3 = vec![
        "I'll check multiple directories.\n",
        "{\"tool\": \"list_files\", \"params\": {\"path\": \".\"}}\n",
        "{\"tool\": \"list_files\", \"params\": {\"path\": \"./src\"}}\n",
        "Done checking.",
    ];
    
    process_pattern(&integration, "Multiple Tools", tokens3).await?;

    println!("\n=== Instrumentation Log ===");
    println!("Check 'full_instrumentation.jsonl' for detailed events");
    println!("Each line is a JSON event with timing and classification data");
    
    // Show sample log analysis
    sleep(Duration::from_millis(500)).await; // Let logs flush
    
    if let Ok(content) = tokio::fs::read_to_string("full_instrumentation.jsonl").await {
        let lines: Vec<&str> = content.lines().collect();
        println!("\nTotal events logged: {}", lines.len());
        
        if let Some(first) = lines.first() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(first) {
                println!("First event type: {}", json.get("event_type").unwrap_or(&serde_json::json!("unknown")));
            }
        }
    }

    Ok(())
}

async fn process_pattern(
    integration: &McpChatIntegration,
    pattern_name: &str,
    tokens: Vec<&str>,
) -> Result<()> {
    println!("\n--- Pattern: {} ---", pattern_name);
    
    let owned_tokens: Vec<String> = tokens.into_iter().map(|s| s.to_string()).collect();
    let stream = stream::iter(owned_tokens);
    let mut handle = integration.process_streaming_response(stream).await?;
    
    let mut narrative_count = 0;
    let mut tool_count = 0;
    let mut buffered_count = 0;
    
    while let Some(token) = handle.token_stream.recv().await {
        match token {
            ProcessedToken::Narrative(_) => narrative_count += 1,
            ProcessedToken::ToolCall(_) => tool_count += 1,
            ProcessedToken::Buffered(_) => buffered_count += 1,
        }
    }
    
    println!("Token classifications:");
    println!("  Narrative: {}", narrative_count);
    println!("  Tool calls: {}", tool_count);
    println!("  Buffered: {}", buffered_count);
    
    let mut tools_executed = 0;
    while let Some(_) = handle.tool_stream.recv().await {
        tools_executed += 1;
    }
    println!("Tools executed: {}", tools_executed);
    
    Ok(())
}