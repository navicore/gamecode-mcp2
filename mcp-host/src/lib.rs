pub mod conversation;
pub mod llm;
pub mod prompts;
pub mod retry;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

pub use conversation::ConversationManager;
pub use llm::{LlmProvider, LlmRequest, LlmResponse, OllamaProvider};
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
            max_tokens_per_request: 100, // Aggressive limit for fast responses
            max_tools_per_request: 5,
            rate_limit_per_minute: 30,
            blocked_tool_patterns: vec![],
        }
    }
}

pub struct McpHost {
    llm_provider: Box<dyn LlmProvider>,
    mcp_client: Arc<Mutex<gamecode_mcp_client::McpClient>>,
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

        let available_tools = self.mcp_client.lock().await.list_tools().await?;
        let mut retry_context = retry::RetryContext::new(0, 0.7);
        let mut final_response = String::new();

        for attempt in 0..=self.config.max_retries {
            retry_context.attempt = attempt;

            let prompt = if attempt == 0 {
                self.build_prompt(&available_tools)?
            } else {
                retry_context.build_retry_prompt(&self.build_prompt(&available_tools)?)
            };

            match self.generate_and_validate_response(&prompt, attempt, user_message).await {
                Ok(response) => {
                    final_response = response;
                    break;
                }
                Err(e) => {
                    tracing::error!("Generate response failed on attempt {}: {}", attempt, e);
                    retry_context.add_error(e.to_string());
                    if attempt < self.config.max_retries {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            self.config.retry_delay_ms,
                        ))
                        .await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(final_response)
    }

    async fn generate_and_validate_response(
        &mut self,
        prompt: &str,
        attempt: u32,
        user_message: &str,
    ) -> Result<String> {
        // Calculate temperature based on retry attempt
        let temperature = self.retry_strategy.calculate_temperature(0.7, attempt);

        tracing::debug!("=== SENDING PROMPT TO LLM ===");
        tracing::debug!("Prompt: {}", prompt);
        tracing::debug!("Temperature: {}, Max tokens: {}", temperature, self.config.safety_constraints.max_tokens_per_request);

        // Generate initial LLM response
        let request = llm::LlmRequest {
            prompt: prompt.to_string(),
            temperature,
            max_tokens: Some(self.config.safety_constraints.max_tokens_per_request as u32),
            stop_sequences: vec!["User:".to_string(), "\n\n\n".to_string()],
        };

        let mut response = self.llm_provider.generate(request).await?;
        
        tracing::debug!("=== LLM RESPONSE ===");
        tracing::debug!("Raw response: {}", response.text);
        
        // Clean up response - remove thinking tags if present
        if let Some(start) = response.text.find("<think>") {
            if let Some(end) = response.text.find("</think>") {
                let before = &response.text[..start];
                let after = &response.text[end + 8..]; // 8 is length of "</think>"
                response.text = format!("{}{}", before, after).trim().to_string();
            }
        }

        // Create template for parsing
        let template = prompts::PromptTemplate::new(self.llm_provider.name());
        
        // Keep track of all tool results across multiple rounds
        let mut all_tool_results: Vec<(String, Value)> = Vec::new();
        let mut current_text = response.text.clone();
        let mut loop_count = 0;
        const MAX_TOOL_ROUNDS: usize = 10;
        
        // Loop to handle multiple rounds of tool calls
        loop {
            loop_count += 1;
            if loop_count > MAX_TOOL_ROUNDS {
                tracing::error!("Tool execution loop exceeded maximum rounds ({})", MAX_TOOL_ROUNDS);
                return Err(anyhow::anyhow!("Tool execution loop exceeded maximum rounds"));
            }
            tracing::debug!("=== TOOL EXECUTION LOOP - ROUND {} ===", loop_count);
            // Validate and check for tool calls
            let validation = template.validate_response(&current_text);
            
            tracing::debug!("=== TOOL CALL VALIDATION ===");
            tracing::debug!("Response validation: has_tool_calls={}, tool_count={}", 
                validation.has_tool_calls, validation.tool_calls.len());
            
            if !validation.has_tool_calls {
                // No more tool calls, return the current response
                return Ok(current_text);
            }
            
            // Check safety constraints
            if validation.tool_calls.len() > self.config.safety_constraints.max_tools_per_request {
                return Err(anyhow::anyhow!(
                    "Too many tool calls ({}) - max allowed: {}",
                    validation.tool_calls.len(),
                    self.config.safety_constraints.max_tools_per_request
                ));
            }
            
            // Execute tool calls
            let mut round_tool_results = Vec::new();
            
            for tool_call in &validation.tool_calls {
                tracing::debug!("Executing tool call: {} with params: {:?}", tool_call.tool, tool_call.params);
                
                match self
                    .mcp_client
                    .lock()
                    .await
                    .call_tool(&tool_call.tool, tool_call.params.clone())
                    .await
                {
                    Ok(result) => {
                        tracing::debug!("Tool {} returned: {:?}", tool_call.tool, result);
                        round_tool_results.push(conversation::ToolResult {
                            tool_name: tool_call.tool.clone(),
                            success: true,
                            result: result.clone(),
                        });
                        all_tool_results.push((tool_call.tool.clone(), result));
                    }
                    Err(e) => {
                        tracing::error!("Tool {} failed: {}", tool_call.tool, e);
                        let error_result = serde_json::json!({ "error": e.to_string() });
                        round_tool_results.push(conversation::ToolResult {
                            tool_name: tool_call.tool.clone(),
                            success: false,
                            result: error_result.clone(),
                        });
                        all_tool_results.push((tool_call.tool.clone(), error_result));
                    }
                }
            }
            
            // Store this round of tool calls in conversation
            self.conversation_manager.add_assistant_message_with_tools(
                &current_text,
                validation.tool_calls,
                round_tool_results,
            );
            
            // Build prompt with all tool results so far
            let available_tools = self.mcp_client.lock().await.list_tools().await?;
            let tool_results_prompt = template.format_tool_results_prompt(
                &available_tools,
                &current_text, // Current text (may have been through multiple rounds)
                &all_tool_results, // All accumulated results
                user_message
            );
            
            tracing::debug!("=== SENDING TOOL RESULTS BACK TO LLM ===");
            tracing::debug!("Total tool results so far: {}", all_tool_results.len());
            tracing::debug!("Tool results prompt preview (first 500 chars): {}", 
                &tool_results_prompt.chars().take(500).collect::<String>());
            
            // Get next response from LLM
            let next_request = llm::LlmRequest {
                prompt: tool_results_prompt,
                temperature: 0.3, // Lower temperature for tool result processing
                max_tokens: Some(self.config.safety_constraints.max_tokens_per_request as u32),
                stop_sequences: vec![],
            };
            
            let mut next_response = self.llm_provider.generate(next_request).await?;
            
            tracing::debug!("=== NEXT LLM RESPONSE ===");
            tracing::debug!("Response: {}", next_response.text);
            
            // Clean up the response
            if let Some(start) = next_response.text.find("<think>") {
                if let Some(end) = next_response.text.find("</think>") {
                    let before = &next_response.text[..start];
                    let after = &next_response.text[end + 8..];
                    next_response.text = format!("{}{}", before, after).trim().to_string();
                }
            }
            
            // Update current_text for the next iteration
            current_text = next_response.text;
            
            // The loop will continue if there are more tool calls
        }
    }

    fn build_prompt(&self, tools: &[gamecode_mcp_client::protocol::Tool]) -> Result<String> {
        let template = prompts::PromptTemplate::new(self.llm_provider.name());
        let history = self.conversation_manager.get_conversation_history();

        tracing::debug!("=== BUILDING PROMPT ==>");
        tracing::debug!("Conversation history has {} messages", history.len());
        for (i, (role, msg)) in history.iter().enumerate() {
            tracing::debug!("Message {}: {} - {}", i, role, msg);
        }

        // Get the last user message (should be the most recent)
        let user_message = history
            .last()
            .filter(|(role, _)| role == "User")
            .map(|(_, msg)| msg.as_str())
            .unwrap_or("");

        let prompt = template.format_with_tools(
            tools,
            &history[..history.len().saturating_sub(1)],
            user_message,
        );
        
        Ok(prompt)
    }
}

#[derive(Default)]
pub struct McpHostBuilder {
    llm_provider: Option<Box<dyn LlmProvider>>,
    mcp_client: Option<Arc<Mutex<gamecode_mcp_client::McpClient>>>,
    config: Option<McpHostConfig>,
}

impl McpHostBuilder {
    pub fn with_llm_provider(mut self, provider: Box<dyn LlmProvider>) -> Self {
        self.llm_provider = Some(provider);
        self
    }

    pub fn with_mcp_client(mut self, client: Arc<Mutex<gamecode_mcp_client::McpClient>>) -> Self {
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
            llm_provider: self
                .llm_provider
                .ok_or_else(|| anyhow::anyhow!("LLM provider required"))?,
            mcp_client: self
                .mcp_client
                .ok_or_else(|| anyhow::anyhow!("MCP client required"))?,
            conversation_manager: ConversationManager::new(
                config.safety_constraints.max_tokens_per_request,
            ),
            retry_strategy: RetryStrategy::new(
                config.max_retries,
                config.retry_delay_ms,
                config.temperature_reduction,
            ),
            config,
        })
    }
}
