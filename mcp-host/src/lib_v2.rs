// Alternative API design focused on chat application integration

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

/// The main interface for chat applications integrating MCP tools
pub struct McpChatEnhancer {
    mcp_client: Arc<Mutex<gamecode_mcp_client::McpClient>>,
    config: McpEnhancerConfig,
    // Separate file logger for debugging without polluting app logs
    debug_logger: Option<Box<dyn DebugLogger>>,
}

#[derive(Debug, Clone)]
pub struct McpEnhancerConfig {
    /// Whether to inject tool descriptions into system prompts
    pub inject_tools_in_system_prompt: bool,
    /// Whether to automatically handle tool calls in responses
    pub auto_execute_tools: bool,
    /// Maximum rounds of tool execution before returning to chat
    pub max_tool_rounds: usize,
    /// Debug log file path (None = no debug logging)
    pub debug_log_path: Option<String>,
}

/// Represents a single message in a chat conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "system", "user", "assistant"
    pub content: String,
    /// Original content before any tool execution (for reconstruction)
    pub original_content: Option<String>,
    /// Tool calls made during this message (if any)
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Tool results (if any)
    pub tool_results: Option<Vec<ToolResult>>,
}

/// Result of processing a message through MCP
#[derive(Debug)]
pub struct ProcessedMessage {
    /// The final message content after all tool executions
    pub final_content: String,
    /// Whether tools were executed
    pub tools_executed: bool,
    /// Tool execution details (for app's context management)
    pub tool_execution_log: Vec<ToolExecutionEntry>,
    /// Tokens used (if available from LLM)
    pub token_usage: Option<TokenUsage>,
}

#[derive(Debug, Clone)]
pub struct ToolExecutionEntry {
    pub tool_name: String,
    pub parameters: Value,
    pub result: Value,
    pub success: bool,
    pub execution_time_ms: u64,
}

impl McpChatEnhancer {
    pub fn new(
        mcp_client: Arc<Mutex<gamecode_mcp_client::McpClient>>,
        config: McpEnhancerConfig,
    ) -> Result<Self> {
        let debug_logger = if let Some(path) = &config.debug_log_path {
            Some(Box::new(FileDebugLogger::new(path)?) as Box<dyn DebugLogger>)
        } else {
            None
        };

        Ok(Self {
            mcp_client,
            config,
            debug_logger,
        })
    }

    /// Enhance a system prompt with available tool descriptions
    /// This preserves the original prompt and appends tool information
    pub async fn enhance_system_prompt(&self, original_prompt: &str) -> Result<String> {
        if !self.config.inject_tools_in_system_prompt {
            return Ok(original_prompt.to_string());
        }

        let tools = self.mcp_client.lock().await.list_tools().await?;
        if tools.is_empty() {
            return Ok(original_prompt.to_string());
        }

        let mut enhanced = original_prompt.to_string();
        enhanced.push_str("\n\n# Available Tools\n");
        enhanced.push_str("You have access to the following tools:\n");
        
        for tool in &tools {
            enhanced.push_str(&format!("- {}: {}\n", tool.name, tool.description));
        }
        
        enhanced.push_str("\nTo use a tool, output a JSON block on its own line:\n");
        enhanced.push_str("{\"tool\": \"tool_name\", \"params\": {\"param\": \"value\"}}\n");

        self.log_debug(&format!("Enhanced system prompt with {} tools", tools.len()));
        
        Ok(enhanced)
    }

    /// Process an LLM response, executing any tool calls found
    /// This is designed to be called AFTER the LLM has generated a response
    pub async fn process_llm_response(
        &self,
        llm_response: &str,
        context: ProcessingContext,
    ) -> Result<ProcessedMessage> {
        self.log_debug(&format!("Processing LLM response: {} chars", llm_response.len()));

        if !self.config.auto_execute_tools {
            return Ok(ProcessedMessage {
                final_content: llm_response.to_string(),
                tools_executed: false,
                tool_execution_log: vec![],
                token_usage: None,
            });
        }

        let mut current_content = llm_response.to_string();
        let mut all_executions = Vec::new();
        let mut round = 0;

        // Extract and execute tool calls in rounds
        while round < self.config.max_tool_rounds {
            round += 1;
            self.log_debug(&format!("Tool execution round {}", round));

            let tool_calls = self.extract_tool_calls(&current_content)?;
            if tool_calls.is_empty() {
                self.log_debug("No tool calls found, finishing");
                break;
            }

            self.log_debug(&format!("Found {} tool calls", tool_calls.len()));

            // Execute tools
            let executions = self.execute_tools(tool_calls).await?;
            all_executions.extend(executions.clone());

            // Get continuation from context's LLM provider
            if let Some(continuation) = context.get_tool_continuation(&executions).await? {
                self.log_debug(&format!("Got continuation: {} chars", continuation.len()));
                current_content = continuation;
            } else {
                // No continuation needed, format results inline
                current_content = self.format_inline_results(&current_content, &executions)?;
                break;
            }
        }

        Ok(ProcessedMessage {
            final_content: current_content,
            tools_executed: !all_executions.is_empty(),
            tool_execution_log: all_executions,
            token_usage: None,
        })
    }

    /// Execute a list of tool calls
    async fn execute_tools(&self, tool_calls: Vec<ToolCall>) -> Result<Vec<ToolExecutionEntry>> {
        let mut executions = Vec::new();

        for call in tool_calls {
            let start = std::time::Instant::now();
            
            let result = self
                .mcp_client
                .lock()
                .await
                .call_tool(&call.tool, call.params.clone())
                .await;

            let execution_time_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(value) => {
                    executions.push(ToolExecutionEntry {
                        tool_name: call.tool.clone(),
                        parameters: call.params,
                        result: value,
                        success: true,
                        execution_time_ms,
                    });
                }
                Err(e) => {
                    executions.push(ToolExecutionEntry {
                        tool_name: call.tool.clone(),
                        parameters: call.params,
                        result: serde_json::json!({"error": e.to_string()}),
                        success: false,
                        execution_time_ms,
                    });
                }
            }
        }

        Ok(executions)
    }

    /// Extract tool calls from LLM response
    fn extract_tool_calls(&self, response: &str) -> Result<Vec<ToolCall>> {
        // Same regex-based extraction as before
        let tool_calls = prompts::extract_tool_calls(response)?;
        Ok(tool_calls)
    }

    /// Format tool results inline in the response
    fn format_inline_results(
        &self,
        original: &str,
        executions: &[ToolExecutionEntry],
    ) -> Result<String> {
        // Simple strategy: append results after tool calls
        let mut result = original.to_string();
        result.push_str("\n\n");
        
        for exec in executions {
            if exec.success {
                result.push_str(&format!("Tool '{}' result: {}\n", 
                    exec.tool_name, 
                    serde_json::to_string_pretty(&exec.result)?
                ));
            } else {
                result.push_str(&format!("Tool '{}' failed: {}\n", 
                    exec.tool_name,
                    exec.result
                ));
            }
        }
        
        Ok(result)
    }

    fn log_debug(&self, message: &str) {
        if let Some(logger) = &self.debug_logger {
            logger.log(message);
        }
    }
}

/// Context provided by the chat application
pub trait ProcessingContext: Send + Sync {
    /// Get a continuation after tool execution
    /// Returns None if no continuation is needed (results should be formatted inline)
    async fn get_tool_continuation(
        &self,
        tool_results: &[ToolExecutionEntry],
    ) -> Result<Option<String>>;
}

/// Simple implementation that doesn't use continuations
pub struct NoContinuationContext;

#[async_trait::async_trait]
impl ProcessingContext for NoContinuationContext {
    async fn get_tool_continuation(
        &self,
        _tool_results: &[ToolExecutionEntry],
    ) -> Result<Option<String>> {
        Ok(None)
    }
}

/// Debug logging trait
trait DebugLogger: Send + Sync {
    fn log(&self, message: &str);
}

/// File-based debug logger
struct FileDebugLogger {
    path: String,
}

impl FileDebugLogger {
    fn new(path: &str) -> Result<Self> {
        Ok(Self {
            path: path.to_string(),
        })
    }
}

impl DebugLogger for FileDebugLogger {
    fn log(&self, message: &str) {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(file, "[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), message);
        }
    }
}

// Re-export types that chat apps will need
pub use prompts::ToolCall;
pub use conversation::ToolResult;
pub use llm::TokenUsage;