pub mod conversation;
pub mod llm;
pub mod prompts;
pub mod retry;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub use conversation::ConversationManager;
pub use llm::{LlmProvider, OllamaProvider};
pub use prompts::PromptTemplate;
pub use retry::RetryStrategy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpHostConfig {
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub temperature_reduction: f32,
    pub safety_constraints: SafetyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub max_tokens_per_request: usize,
    pub max_tools_per_request: usize,
    pub rate_limit_per_minute: u32,
    pub blocked_tool_patterns: Vec<String>,
}

impl Default for McpHostConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 500,
            temperature_reduction: 0.1,
            safety_constraints: SafetyConfig::default(),
        }
    }
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            max_tokens_per_request: 4096,
            max_tools_per_request: 5,
            rate_limit_per_minute: 30,
            blocked_tool_patterns: vec![],
        }
    }
}

pub struct McpHost {
    llm_provider: Box<dyn LlmProvider>,
    mcp_client: Arc<mcp_client::McpClient>,
    config: McpHostConfig,
    conversation_manager: ConversationManager,
    retry_strategy: RetryStrategy,
}

impl McpHost {
    pub fn builder() -> McpHostBuilder {
        McpHostBuilder::default()
    }

    pub async fn process_message(&mut self, user_message: &str) -> Result<String> {
        self.conversation_manager.add_user_message(user_message);
        
        let available_tools = self.mcp_client.list_tools().await?;
        let mut retry_context = retry::RetryContext::new(0, 0.7);
        let mut final_response = String::new();
        
        for attempt in 0..=self.config.max_retries {
            retry_context.attempt = attempt;
            
            let prompt = if attempt == 0 {
                self.build_prompt(&available_tools)?
            } else {
                retry_context.build_retry_prompt(&self.build_prompt(&available_tools)?)
            };
            
            match self.generate_and_validate_response(&prompt, attempt).await {
                Ok(response) => {
                    final_response = response;
                    break;
                }
                Err(e) => {
                    retry_context.add_error(e.to_string());
                    if attempt < self.config.max_retries {
                        tokio::time::sleep(
                            std::time::Duration::from_millis(self.config.retry_delay_ms)
                        ).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        
        if !final_response.is_empty() {
            self.conversation_manager.add_assistant_message(&final_response);
        }
        
        Ok(final_response)
    }

    async fn generate_and_validate_response(&mut self, prompt: &str, attempt: u32) -> Result<String> {
        // Calculate temperature based on retry attempt
        let temperature = self.retry_strategy.calculate_temperature(0.7, attempt);
        
        // Generate LLM response
        let request = llm::LlmRequest {
            prompt: prompt.to_string(),
            temperature,
            max_tokens: Some(self.config.safety_constraints.max_tokens_per_request as u32),
            stop_sequences: vec![],
        };
        
        let response = self.llm_provider.generate(request).await?;
        
        // Validate and process tool calls
        let template = prompts::PromptTemplate::new(self.llm_provider.name());
        let validation = template.validate_response(&response.text);
        
        if validation.has_tool_calls {
            // Check safety constraints
            if validation.tool_calls.len() > self.config.safety_constraints.max_tools_per_request {
                return Err(anyhow::anyhow!(
                    "Too many tool calls ({}) - max allowed: {}", 
                    validation.tool_calls.len(),
                    self.config.safety_constraints.max_tools_per_request
                ));
            }
            
            // Execute tool calls
            let mut tool_results = Vec::new();
            let mut response_text = String::new();
            
            for tool_call in &validation.tool_calls {
                match self.mcp_client.call_tool(&tool_call.tool, tool_call.params.clone()).await {
                    Ok(result) => {
                        tool_results.push(conversation::ToolResult {
                            tool_name: tool_call.tool.clone(),
                            success: true,
                            result,
                        });
                        response_text.push_str(&format!(
                            "Tool '{}' executed successfully.\n", 
                            tool_call.tool
                        ));
                    }
                    Err(e) => {
                        tool_results.push(conversation::ToolResult {
                            tool_name: tool_call.tool.clone(),
                            success: false,
                            result: serde_json::json!({ "error": e.to_string() }),
                        });
                        response_text.push_str(&format!(
                            "Tool '{}' failed: {}\n", 
                            tool_call.tool, e
                        ));
                    }
                }
            }
            
            // Store tool calls and results in conversation
            self.conversation_manager.add_assistant_message_with_tools(
                &response_text,
                validation.tool_calls,
                tool_results,
            );
            
            Ok(response_text)
        } else if validation.is_valid {
            Ok(response.text)
        } else {
            Err(anyhow::anyhow!("Invalid response format - no valid content or tool calls"))
        }
    }

    fn build_prompt(&self, tools: &[mcp_client::protocol::Tool]) -> Result<String> {
        let template = prompts::PromptTemplate::new(self.llm_provider.name());
        let history = self.conversation_manager.get_conversation_history();
        
        // Get the last user message (should be the most recent)
        let user_message = history.last()
            .filter(|(role, _)| role == "User")
            .map(|(_, msg)| msg.as_str())
            .unwrap_or("");
        
        Ok(template.format_with_tools(tools, &history[..history.len().saturating_sub(1)], user_message))
    }
}

#[derive(Default)]
pub struct McpHostBuilder {
    llm_provider: Option<Box<dyn LlmProvider>>,
    mcp_client: Option<Arc<mcp_client::McpClient>>,
    config: Option<McpHostConfig>,
}

impl McpHostBuilder {
    pub fn with_llm_provider(mut self, provider: Box<dyn LlmProvider>) -> Self {
        self.llm_provider = Some(provider);
        self
    }

    pub fn with_mcp_client(mut self, client: Arc<mcp_client::McpClient>) -> Self {
        self.mcp_client = Some(client);
        self
    }

    pub fn with_config(mut self, config: McpHostConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn build(self) -> Result<McpHost> {
        let config = self.config.unwrap_or_default();
        
        Ok(McpHost {
            llm_provider: self.llm_provider
                .ok_or_else(|| anyhow::anyhow!("LLM provider required"))?,
            mcp_client: self.mcp_client
                .ok_or_else(|| anyhow::anyhow!("MCP client required"))?,
            conversation_manager: ConversationManager::new(config.safety_constraints.max_tokens_per_request),
            retry_strategy: RetryStrategy::new(
                config.max_retries,
                config.retry_delay_ms,
                config.temperature_reduction,
            ),
            config,
        })
    }
}