/// Example of a chat application using mcp-host with streaming
use anyhow::Result;
use futures::{stream, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

// Simulated chat application components
struct ChatApp {
    mcp_integration: McpChatIntegration,
    ollama_client: OllamaClient,
    ui: ChatUI,
}

impl ChatApp {
    async fn new() -> Result<Self> {
        // Initialize MCP client
        let mut mcp_client = gamecode_mcp_client::McpClient::connect(
            "target/debug/gamecode-mcp2",
            &["--tools-file".to_string(), "tools.yaml".to_string()],
        ).await?;
        mcp_client.initialize("streaming-chat-app", "1.0.0").await?;
        let mcp_client = Arc::new(Mutex::new(mcp_client));

        // Configure MCP integration
        let config = ChatIntegrationConfig {
            streaming_mode: StreamingMode::SmartBuffering {
                max_buffer_chars: 150,
            },
            enhance_system_prompts: true,
            max_tool_rounds: 3,
            instrumentation: InstrumentationConfig {
                log_path: Some("chat_mcp_debug.log".to_string()),
                log_token_classifications: true,
                log_performance_metrics: true,
            },
        };

        let mcp_integration = McpChatIntegration::new(mcp_client, config).await?;
        let ollama_client = OllamaClient::new("magistral:24b");
        let ui = ChatUI::new();

        Ok(Self {
            mcp_integration,
            ollama_client,
            ui,
        })
    }

    async fn run(&mut self) -> Result<()> {
        // Initial system prompt
        let base_system_prompt = "You are a helpful AI assistant. Be concise and accurate.";
        let enhanced_prompt = self.mcp_integration
            .enhance_system_prompt(base_system_prompt)
            .await?;

        // Start conversation
        let mut messages = vec![
            Message { role: "system", content: enhanced_prompt },
        ];

        // Main chat loop
        loop {
            // Get user input
            let user_input = self.ui.get_user_input().await?;
            if user_input == "/exit" {
                break;
            }

            messages.push(Message {
                role: "user",
                content: user_input.clone(),
            });

            // Show typing indicator
            self.ui.show_typing_indicator();

            // Get streaming response from Ollama
            let token_stream = self.ollama_client
                .chat_streaming(&messages)
                .await?;

            // Process through MCP integration
            let mut handle = self.mcp_integration
                .process_streaming_response(token_stream)
                .await?;

            // Collect the complete response for conversation history
            let mut complete_response = String::new();
            let mut tools_used = Vec::new();

            // Display tokens as they arrive
            while let Some(token) = handle.token_stream.recv().await {
                match token {
                    ProcessedToken::Narrative(text) => {
                        self.ui.append_assistant_text(&text).await;
                        complete_response.push_str(&text);
                    }
                    ProcessedToken::ToolCall(_) => {
                        // Hidden from user, but we track it
                    }
                    ProcessedToken::Buffered(_) => {
                        // Still being analyzed
                    }
                }
            }

            // Handle any tools that were executed
            while let Some(tool) = handle.tool_stream.recv().await {
                tools_used.push(tool.tool_name.clone());
                
                // Optionally show tool usage indicator
                self.ui.show_tool_indicator(&format!(
                    "Used tool: {} ({}ms)",
                    tool.tool_name,
                    tool.execution_time_ms
                )).await;

                // The tool results are already incorporated in the response
            }

            // Add complete response to conversation history
            messages.push(Message {
                role: "assistant",
                content: complete_response,
            });

            self.ui.finish_response();
        }

        Ok(())
    }
}

// Mock Ollama client that supports streaming
struct OllamaClient {
    model: String,
}

impl OllamaClient {
    fn new(model: &str) -> Self {
        Self { model: model.to_string() }
    }

    async fn chat_streaming(
        &self,
        messages: &[Message],
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = String> + Send>>> {
        // In real implementation, this would stream from Ollama API
        // For demo, we'll simulate a response with tool usage
        
        let tokens = if messages.last().unwrap().content.contains("files") {
            // Response that includes a tool call
            vec![
                "I'll ",
                "check ",
                "the ",
                "files ",
                "for ",
                "you",
                ".\n\n",
                "{\"tool\": \"list_files\", \"params\": {\"path\": \".\"}}",
                "\n\n",
                "Here ",
                "are ",
                "the ",
                "files ",
                "in ",
                "the ",
                "current ",
                "directory",
                ":",
            ]
        } else {
            // Normal response
            vec![
                "I ",
                "understand",
                ". ",
                "How ",
                "can ",
                "I ",
                "help ",
                "you ",
                "today",
                "?",
            ]
        };

        // Simulate streaming with delays
        let owned_tokens: Vec<String> = tokens.into_iter().map(|s| s.to_string()).collect();
        Ok(Box::pin(stream::iter(owned_tokens)
            .then(|token| async move {
                sleep(Duration::from_millis(50)).await; // Simulate network delay
                token
            })))
    }
}

// Mock UI component
struct ChatUI {
    // In real app, this would handle terminal/web UI
}

impl ChatUI {
    fn new() -> Self {
        Self {}
    }

    async fn get_user_input(&self) -> Result<String> {
        use std::io::{self, Write};
        
        print!("\n> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    fn show_typing_indicator(&self) {
        print!("\nAssistant: ");
        use std::io::{self, Write};
        let _ = io::stdout().flush();
    }

    async fn append_assistant_text(&self, text: &str) {
        print!("{}", text);
        use std::io::{self, Write};
        let _ = io::stdout().flush();
    }

    async fn show_tool_indicator(&self, message: &str) {
        println!("\n[{}]", message);
    }

    fn finish_response(&self) {
        println!(); // New line after response
    }
}

#[derive(Clone)]
struct Message {
    role: &'static str,
    content: String,
}

// Import types from mcp-host
use gamecode_mcp_host::{
    McpChatIntegration, ChatIntegrationConfig, StreamingMode,
    InstrumentationConfig, ProcessedToken,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Streaming Chat Application with MCP Tools");
    println!("========================================");
    println!("Type '/exit' to quit\n");

    let mut app = ChatApp::new().await?;
    app.run().await?;

    Ok(())
}