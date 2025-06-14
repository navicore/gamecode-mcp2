use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

// This example shows how a chat application would integrate mcp-host
// without giving up control of its LLM interaction

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the chat app's own Ollama client
    let ollama_client = OllamaClient::new("magistral:24b");
    
    // Initialize MCP client for tools
    let mut mcp_client = gamecode_mcp_client::McpClient::connect(
        "target/debug/gamecode-mcp2",
        &["--tools-file".to_string(), "tools.yaml".to_string()],
    ).await?;
    mcp_client.initialize("chat-app", "1.0.0").await?;
    let mcp_client = Arc::new(Mutex::new(mcp_client));
    
    // Create MCP enhancer with debug logging
    let enhancer = McpChatEnhancer::new(
        mcp_client,
        McpEnhancerConfig {
            inject_tools_in_system_prompt: true,
            auto_execute_tools: true,
            max_tool_rounds: 3,
            debug_log_path: Some("mcp_debug.log".to_string()),
        },
    )?;
    
    // The chat app's system prompt
    let app_system_prompt = "You are a helpful assistant. Be concise and friendly.";
    
    // Enhance with available tools
    let enhanced_prompt = enhancer.enhance_system_prompt(app_system_prompt).await?;
    
    // Chat conversation (managed by the app)
    let mut messages = vec![
        Message { role: "system", content: enhanced_prompt },
        Message { role: "user", content: "What files are in the current directory?".to_string() },
    ];
    
    // The app calls its LLM (with its own context management, streaming, etc.)
    println!("Calling LLM with enhanced prompt...");
    let llm_response = ollama_client.chat(&messages).await?;
    println!("LLM response: {}", llm_response);
    
    // Process the response for tool calls
    let processed = enhancer.process_llm_response(
        &llm_response,
        NoContinuationContext,
    ).await?;
    
    if processed.tools_executed {
        println!("Tools were executed!");
        println!("Final response: {}", processed.final_content);
        
        // The app decides how to handle this
        // Option 1: Use the final content as the assistant message
        messages.push(Message {
            role: "assistant",
            content: processed.final_content,
        });
        
        // Option 2: Store tool execution details separately
        // for own context management
    } else {
        // No tools, use original response
        messages.push(Message {
            role: "assistant", 
            content: llm_response,
        });
    }
    
    // Continue conversation...
    
    Ok(())
}

// Mock types representing the chat app's existing code
struct OllamaClient {
    model: String,
}

impl OllamaClient {
    fn new(model: &str) -> Self {
        Self { model: model.to_string() }
    }
    
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        // The app's existing Ollama integration
        // with its own streaming, context management, etc.
        Ok("I'll list the files for you.\n{\"tool\": \"list_files\", \"params\": {\"path\": \".\"}}".to_string())
    }
}

#[derive(Clone)]
struct Message {
    role: &'static str,
    content: String,
}

// Import the new API (this would be from mcp-host)
use lib_v2::{McpChatEnhancer, McpEnhancerConfig, NoContinuationContext};